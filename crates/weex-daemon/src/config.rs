//! Daemon configuration from environment variables

use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// MySQL database connection URL
    pub database_url: String,

    /// Archive interval in seconds (default: 300 = 5 minutes)
    pub archive_interval: i32,

    /// Poll interval for driver in seconds (default: 10)
    pub poll_interval: u64,

    /// Unit system (1=US, 16=Metric, 17=MetricWX)
    pub unit_system: i32,

    /// Station driver type
    #[allow(dead_code)]
    pub driver: String,
}

impl DaemonConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let database_url =
            env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;

        let archive_interval = env::var("ARCHIVE_INTERVAL")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .context("Invalid ARCHIVE_INTERVAL")?;

        let poll_interval = env::var("POLL_INTERVAL")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .context("Invalid POLL_INTERVAL")?;

        let unit_system = env::var("UNIT_SYSTEM")
            .unwrap_or_else(|_| "16".to_string()) // Default to Metric
            .parse()
            .context("Invalid UNIT_SYSTEM")?;

        let driver = env::var("STATION_DRIVER").unwrap_or_else(|_| "simulator".to_string());

        Ok(Self {
            database_url,
            archive_interval,
            poll_interval,
            unit_system,
            driver,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Test that reasonable defaults exist
        env::set_var("DATABASE_URL", "mysql://localhost/weewx");

        let config = DaemonConfig::from_env().unwrap();

        assert_eq!(config.archive_interval, 300);
        assert_eq!(config.poll_interval, 10);
        assert_eq!(config.unit_system, 16);
        assert_eq!(config.driver, "simulator");

        env::remove_var("DATABASE_URL");
    }
}
