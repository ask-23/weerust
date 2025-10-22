use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tower::ServiceExt;

#[tokio::test]
async fn udp_packet_populates_api() {
    let (app, state) = weewx_cli::build_app();
    // Bind to ephemeral port
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (local, _handle) = weewx_cli::start_interceptor_ingest(state.clone(), bind, None)
        .await
        .unwrap();

    // Send a JSON WeatherPacket over UDP
    let sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let json = r#"{"dateTime":1700000000,"station":"gw1100","interval":5,"outTemp":21.5}"#;
    sock.send_to(json.as_bytes(), local).await.unwrap();

    // Eventually should appear as current
    // Note: small retry loop in case of scheduling delay
    for _ in 0..10 {
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
        if res.status() == StatusCode::OK {
            let body = to_bytes(res.into_body()).await.unwrap();
            let text = String::from_utf8(body.to_vec()).unwrap();
            if text.contains("outTemp") {
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("current did not populate from UDP packet");
}
