use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use opentelemetry::metrics::Counter;
use opentelemetry_prometheus::exporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, Registry, TextEncoder};
use serde::Deserialize;
use tokio::sync::Mutex;
use weex_core::WeatherPacket;

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
        history: Mutex::new(Vec::with_capacity(HISTORY_CAP)),
    });

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .route("/api/v1/current", get(current))
        .route("/api/v1/history", get(history))
        .with_state(Arc::clone(&state));

    (router, state)
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
