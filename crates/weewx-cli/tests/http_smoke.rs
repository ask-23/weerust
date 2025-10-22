use std::collections::HashMap;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use weex_core::{ObservationValue, WeatherPacket};
use tower::ServiceExt;

#[tokio::test]
async fn health_ready_metrics_endpoints() {
    let (app, state) = weewx_cli::build_app();

    // /healthz returns 200 and increments a counter
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // /readyz initially 503
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Set ready
    weewx_cli::set_ready(&state, true);

    // /readyz now 200
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // /metrics returns prometheus text and contains our counter
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.starts_with("text/plain"));
    let body = to_bytes(res.into_body()).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("weewx_requests_total"));
}

#[tokio::test]
async fn history_endpoint_respects_limit() {
    let (app, state) = weewx_cli::build_app();

    for i in 0..3 {
        let mut observations = HashMap::new();
        observations.insert(
            "outTemp".to_string(),
            ObservationValue::Float(70.0 + f64::from(i)),
        );
        let ts = i64::from(i) + 1;
        let packet = WeatherPacket {
            date_time: ts,
            station: None,
            interval: None,
            observations,
        };
        weewx_cli::inject_packet(&state, packet).await;
    }

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/history?limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body()).await.unwrap();
    let packets: Vec<WeatherPacket> = serde_json::from_slice(&body).unwrap();
    assert_eq!(packets.len(), 2);
    assert!(packets.iter().all(|pkt| pkt.date_time >= 2));
}
