use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use weex_core::{ObservationValue, WeatherPacket};

#[tokio::test]
async fn current_and_history_endpoints() {
    let (app, state) = weewx_cli::build_app();

    // Initially no data => current is 204
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Inject a packet
    let mut obs = std::collections::HashMap::new();
    obs.insert("outTemp".to_string(), ObservationValue::Float(22.2));
    let pkt = WeatherPacket {
        date_time: 1_700_000_001,
        station: Some("sim".into()),
        interval: Some(5),
        observations: obs,
    };
    weewx_cli::inject_packet(&state, pkt).await;

    // current now returns JSON
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body()).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("\"outTemp\""));

    // history returns at least one
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/history?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body()).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.starts_with("["));
}
