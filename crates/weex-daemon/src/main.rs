//! WeeWX Daemon - Main scheduler and archive writer
//!
//! This binary coordinates:
//! - Weather station data collection (via drivers)
//! - Interval aggregation
//! - Archive record writing to MySQL

mod config;
mod scheduler;

use anyhow::{Context, Result};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use weex_archive::IntervalAggregator;
use weex_db::{DbClient, DbConnectionBuilder};
use weex_ingest::simulator::SimulatorDriver;
use weex_ingest::StationDriver;

use crate::config::DaemonConfig;
use crate::scheduler::Scheduler;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting WeeWX Rust Daemon");

    // Load configuration
    let config = DaemonConfig::from_env()?;
    info!("Loaded configuration: {:?}", config);

    // Initialize database connection
    let db_client = DbClient::new(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    info!("Connected to database");

    // Test database connection
    db_client.ping().await.context("Database ping failed")?;
    info!("Database connection verified");

    // Initialize station driver (simulator for now)
    let mut driver = Box::new(SimulatorDriver::new(config.poll_interval)) as Box<dyn StationDriver>;
    driver.start().await.context("Failed to start driver")?;
    info!("Station driver started: {}", driver.name());

    // Create aggregator
    let aggregator = IntervalAggregator::new(
        config.archive_interval,
        config.unit_system,
        db_client.clone(),
    );

    // Create and run scheduler
    let mut scheduler = Scheduler::new(driver, aggregator);

    // Setup signal handler for graceful shutdown
    let shutdown = setup_shutdown_handler();

    info!("Daemon running - press Ctrl+C to stop");

    // Run until shutdown signal
    tokio::select! {
        result = scheduler.run() => {
            if let Err(e) = result {
                error!("Scheduler error: {}", e);
                return Err(e.into());
            }
        }
        _ = shutdown => {
            info!("Shutdown signal received");
            scheduler.stop().await?;
        }
    }

    info!("WeeWX Daemon stopped");
    Ok(())
}

/// Setup graceful shutdown handler
async fn setup_shutdown_handler() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to setup signal handler");
}
