use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Observability
    weewx_obs::init("weewx-rs");

    // Config
    let cfg = weewx_config::AppConfig::load().unwrap_or_default();
    let http_bind = cfg.http_bind();
    let udp_bind = cfg.interceptor_bind();
    let fs_dir = cfg.fs_dir();

    // Build app and state
    let (app, state) = weewx_cli::build_app();

    // Start UDP ingest in background
    let udp_addr: SocketAddr = udp_bind.parse().expect("Invalid UDP bind address");
    match weewx_cli::start_interceptor_ingest(state.clone(), udp_addr, fs_dir).await {
        Ok((local, _handle)) => tracing::info!(%local, "INTERCEPTOR UDP ingest listening"),
        Err(e) => tracing::error!(error=?e, "failed to start UDP ingest"),
    }

    // Start HTTP server
    let addr: SocketAddr = http_bind.parse().expect("Invalid HTTP bind address");
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    // Mark ready just before serving
    weewx_cli::set_ready(&state, true);

    tracing::info!(%addr, "HTTP server listening");
    axum::serve(listener, app).await.expect("server error");
}
