use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Form, Json, Router,
};
use opentelemetry::metrics::{Counter, MeterProvider};
use opentelemetry_prometheus::exporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, Registry, TextEncoder};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use weewx_sinks::FsSink;
use weex_core::{Sink, WeatherPacket};
use weex_ingest::{InterceptorUdpDriver, StationDriver};

const HISTORY_CAP: usize = 1000;

pub struct AppState {
    ready: AtomicBool,
    registry: Registry,
    #[allow(dead_code)]
    provider: SdkMeterProvider,
    requests_total: Counter<u64>,
    latest: Mutex<Option<WeatherPacket>>,
    history: Mutex<Vec<WeatherPacket>>,
}

pub fn build_app() -> (Router, Arc<AppState>) {
    // Prometheus exporter via OpenTelemetry
    let registry = Registry::new();
    let reader = exporter()
        .with_registry(registry.clone())
        .build()
        .expect("prom exporter");
    let provider = SdkMeterProvider::builder().with_reader(reader).build();
    let meter = provider.meter("weewx-cli");

    let requests_total = meter
        .u64_counter("weewx_requests_total")
        .with_description("Total HTTP requests served")
        .init();

    let state = Arc::new(AppState {
        ready: AtomicBool::new(false),
        registry,
        provider,
        requests_total,
        latest: Mutex::new(None),
        history: Mutex::new(Vec::with_capacity(256)),
    });

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .route("/api/v1/current", get(current))
        .route("/api/v1/history", get(history))
        .route("/ingest/ecowitt", get(ingest_ecowitt).post(ingest_post))
        .route("/data", post(ingest_post))
        .with_state(Arc::clone(&state));

    (router, state)
}

pub async fn start_interceptor_ingest(
    state: Arc<AppState>,
    bind: SocketAddr,
    fs_dir: Option<String>,
) -> Result<(SocketAddr, JoinHandle<()>)> {
    let (tx, rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let mut driver = InterceptorUdpDriver::new(bind);
        if let Err(e) = driver.start().await {
            tracing::error!(error=?e, "failed to start interceptor driver");
            let _ = tx.send(Err(e.into()));
            return;
        }
        // Report bound address
        let _ = tx.send(Ok(bind));

        let mut fs_sink = match fs_dir {
            Some(dir) => match FsSink::new(dir) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::warn!(error=?e, "fs sink disabled");
                    None
                }
            },
            None => None,
        };

        loop {
            match driver.get_packet().await {
                Ok(pkt) => {
                    inject_packet(&state, pkt.clone()).await;
                    if let Some(sink) = fs_sink.as_mut() {
                        let _ = sink.emit(&pkt).await;
                    }
                }
                Err(e) => {
                    tracing::warn!(error=?e, "ingest error");
                }
            }
        }
    });
    let local = rx
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("driver start channel closed")))?;
    Ok((local, handle))
}

pub fn set_ready(state: &Arc<AppState>, is_ready: bool) {
    state.ready.store(is_ready, Ordering::Relaxed);
}

pub async fn inject_packet(state: &Arc<AppState>, packet: WeatherPacket) {
    {
        let mut latest = state.latest.lock().await;
        *latest = Some(packet.clone());
    }
    let mut hist = state.history.lock().await;
    hist.push(packet);
    if hist.len() > HISTORY_CAP {
        let overflow = hist.len() - HISTORY_CAP;
        hist.drain(0..overflow);
    }
}

async fn healthz(State(state): State<Arc<AppState>>) -> StatusCode {
    state.requests_total.add(1, &[]);
    StatusCode::OK
}

async fn readyz(State(state): State<Arc<AppState>>) -> StatusCode {
    if state.ready.load(Ordering::Relaxed) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn metrics(
    State(state): State<Arc<AppState>>,
) -> (
    [(axum::http::header::HeaderName, axum::http::HeaderValue); 1],
    String,
) {
    let encoder = TextEncoder::new();
    let metric_families = state.registry.gather();
    let mut buf = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        tracing::warn!(error=?e, "failed to encode metrics");
    }
    let body = String::from_utf8(buf).unwrap_or_default();
    let header = (
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    ([header], body)
}

async fn current(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let latest = state.latest.lock().await;
    if let Some(pkt) = latest.as_ref() {
        return (StatusCode::OK, Json(pkt)).into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
}

async fn history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100).min(HISTORY_CAP);
    let hist = state.history.lock().await;
    let start = hist.len().saturating_sub(limit);
    let slice = hist[start..].to_vec();
    (StatusCode::OK, Json(slice)).into_response()
}

use std::collections::HashMap;
use weex_core::ObservationValue;

async fn ingest_ecowitt(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    state.requests_total.add(1, &[]);
    // dateutc can be "now" or "YYYY-MM-DD HH:M:SS" (UTC)
    let date_time = match q.get("dateutc").map(|s| s.as_str()) {
        Some("now") | None => chrono::Utc::now().timestamp(),
        Some(s) => chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|naive| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc)
                    .timestamp()
            })
            .unwrap_or_else(|| chrono::Utc::now().timestamp()),
    };

    let station = q.get("stationtype").cloned();
    let mut obs: HashMap<String, ObservationValue> = HashMap::new();

    // Helpers
    let parse_f64 = |k: &str| q.get(k).and_then(|v| v.parse::<f64>().ok());
    let parse_i64 = |k: &str| q.get(k).and_then(|v| v.parse::<i64>().ok());

    // Temperature: tempf (F) -> outTemp (C)
    if let Some(tf) = parse_f64("tempf") {
        let c = (tf - 32.0) * (5.0 / 9.0);
        obs.insert("outTemp".into(), ObservationValue::Float(c));
    }
    // Humidity (% RH)
    if let Some(h) = parse_i64("humidity") {
        obs.insert("humidity".into(), ObservationValue::Integer(h));
    }
    // Barometer: baromin (inHg) -> hPa
    if let Some(inhg) = parse_f64("baromin") {
        let hpa = inhg * 33.8638866667;
        obs.insert("barometer".into(), ObservationValue::Float(hpa));
    }
    // Wind: mph -> m/s
    if let Some(mph) = parse_f64("windspeedmph") {
        let mps = mph * 0.44704;
        obs.insert("windSpeed".into(), ObservationValue::Float(mps));
    }
    if let Some(mph) = parse_f64("windgustmph") {
        let mps = mph * 0.44704;
        obs.insert("windGust".into(), ObservationValue::Float(mps));
    }
    if let Some(dir) = parse_i64("winddir") {
        obs.insert("windDir".into(), ObservationValue::Integer(dir));
    }
    // Rain: inches -> mm
    if let Some(rri) = parse_f64("rainin") {
        obs.insert("rainRate".into(), ObservationValue::Float(rri * 25.4));
    }
    if let Some(dri) = parse_f64("dailyrainin") {
        obs.insert("dailyRain".into(), ObservationValue::Float(dri * 25.4));
    }
    // Solar / UV
    if let Some(sr) = parse_f64("solarradiation") {
        obs.insert("radiation".into(), ObservationValue::Float(sr));
    }
    if let Some(uv) = parse_f64("uv") {
        obs.insert("uv".into(), ObservationValue::Float(uv));
    }

    let packet = WeatherPacket {
        date_time,
        station,
        interval: None,
        observations: obs,
    };

    inject_packet(&state, packet).await;

    // TODO: Optionally emit to sinks (Fs/Sqlite/Postgres/Influx) once shared sink wiring is added to AppState

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}

async fn ingest_post(
    State(state): State<Arc<AppState>>,
    Form(q): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    state.requests_total.add(1, &[]);
    // dateutc can be "now" or "YYYY-MM-DD HH:MM:SS" (UTC)
    let date_time = match q.get("dateutc").map(|s| s.as_str()) {
        Some("now") | None => chrono::Utc::now().timestamp(),
        Some(s) => chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|naive| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc)
                    .timestamp()
            })
            .unwrap_or_else(|| chrono::Utc::now().timestamp()),
    };

    let station = q.get("stationtype").cloned();
    let mut obs: HashMap<String, ObservationValue> = HashMap::new();

    // Helpers
    let parse_f64 = |k: &str| q.get(k).and_then(|v| v.parse::<f64>().ok());
    let parse_i64 = |k: &str| q.get(k).and_then(|v| v.parse::<i64>().ok());

    // Temperature: tempf (F) -> outTemp (C)
    if let Some(tf) = parse_f64("tempf") {
        let c = (tf - 32.0) * (5.0 / 9.0);
        obs.insert("outTemp".into(), ObservationValue::Float(c));
    }
    // Humidity (% RH)
    if let Some(h) = parse_i64("humidity") {
        obs.insert("humidity".into(), ObservationValue::Integer(h));
    }
    // Barometer: baromin (inHg) -> hPa
    if let Some(inhg) = parse_f64("baromin") {
        let hpa = inhg * 33.8638866667;
        obs.insert("barometer".into(), ObservationValue::Float(hpa));
    }
    // Also handle baromabsin and baromrelin
    if let Some(inhg) = parse_f64("baromabsin") {
        let hpa = inhg * 33.8638866667;
        obs.insert("barometerAbs".into(), ObservationValue::Float(hpa));
    }
    if let Some(inhg) = parse_f64("baromrelin") {
        let hpa = inhg * 33.8638866667;
        obs.insert("barometer".into(), ObservationValue::Float(hpa));
    }
    // Wind: mph -> m/s
    if let Some(mph) = parse_f64("windspeedmph") {
        let mps = mph * 0.44704;
        obs.insert("windSpeed".into(), ObservationValue::Float(mps));
    }
    if let Some(mph) = parse_f64("windgustmph") {
        let mps = mph * 0.44704;
        obs.insert("windGust".into(), ObservationValue::Float(mps));
    }
    if let Some(dir) = parse_i64("winddir") {
        obs.insert("windDir".into(), ObservationValue::Integer(dir));
    }
    // Rain: inches -> mm
    if let Some(rri) = parse_f64("rainin") {
        obs.insert("rainRate".into(), ObservationValue::Float(rri * 25.4));
    }
    if let Some(dri) = parse_f64("dailyrainin") {
        obs.insert("dailyRain".into(), ObservationValue::Float(dri * 25.4));
    }
    // Solar / UV
    if let Some(sr) = parse_f64("solarradiation") {
        obs.insert("radiation".into(), ObservationValue::Float(sr));
    }
    if let Some(uv) = parse_f64("uv") {
        obs.insert("uv".into(), ObservationValue::Float(uv));
    }

    let packet = WeatherPacket {
        date_time,
        station,
        interval: None,
        observations: obs,
    };

    inject_packet(&state, packet).await;

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}
