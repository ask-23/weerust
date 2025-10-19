use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Observability
    weewx_obs::init("weewx-rs");

    // Config
    let cfg = weewx_config::AppConfig::load().unwrap_or_default();
    let bind = cfg.http_bind();

    // Build app and state
    let (app, state) = weewx_cli::build_app();

    let addr: SocketAddr = bind.parse().expect("Invalid bind address");
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    // Mark ready just before serving
    weewx_cli::set_ready(&state, true);

    tracing::info!(%addr, "HTTP server listening");
    axum::serve(listener, app).await.expect("server error");
}
