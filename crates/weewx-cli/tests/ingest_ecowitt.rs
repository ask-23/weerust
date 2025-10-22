use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn ecowitt_upload_populates_api() {
    let (app, state) = weewx_cli::build_app();
    // Simulate Ecowitt GET upload
    let uri = "/ingest/ecowitt?PASSKEY=ABC&stationtype=GW1100&dateutc=now&tempf=72.5&baromin=29.92&humidity=55&windspeedmph=5.0&windgustmph=7.0&winddir=180";
    let res = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify current has populated fields
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
    assert!(text.contains("outTemp"));
    assert!(text.contains("barometer"));
    assert!(text.contains("windSpeed"));
}
