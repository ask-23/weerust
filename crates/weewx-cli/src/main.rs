use axum::http::StatusCode;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Observability
    weewx_obs::init("weewx-rs");

    // Config
    let cfg = weewx_config::AppConfig::load().unwrap_or_default();
    let bind = cfg.http_bind();

    // HTTP routes
    let app = Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/readyz", get(|| async { StatusCode::OK }))
        .route("/metrics", get(metrics_handler));

    let addr: SocketAddr = bind.parse().expect("Invalid bind address");
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");
    tracing::info!(%addr, "HTTP server listening");
    axum::serve(listener, app).await.expect("server error");
}

async fn metrics_handler() -> (
    [(axum::http::header::HeaderName, axum::http::HeaderValue); 1],
    String,
) {
    let header = (
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    let body =
        "# HELP weewx_rs_dummy 1\n# TYPE weewx_rs_dummy counter\nweewx_rs_dummy 1\n".to_string();
    ([header], body)
}
