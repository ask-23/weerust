use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize observability (logging/tracing). Placeholder for full OTel wiring.
/// - JSON logs, Cloud Logging friendly
/// - RUST_LOG respected; default to "info,weewx=debug"
pub fn init(service_name: &str) {
    let default_filter = "info,weewx=debug";
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| default_filter.to_string());

    tracing_subscriber::registry()
        .with(EnvFilter::new(env_filter))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!(service = %service_name, "Observability initialized");
}
