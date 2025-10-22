//! Packet collection and archiving scheduler

use anyhow::{Context, Result};
use tracing::{error, info, warn};
use weex_archive::IntervalAggregator;
use weex_ingest::StationDriver;

/// Scheduler coordinates data collection and archiving
pub struct Scheduler {
    driver: Box<dyn StationDriver>,
    aggregator: IntervalAggregator,
    running: bool,
}

impl Scheduler {
    pub fn new(driver: Box<dyn StationDriver>, aggregator: IntervalAggregator) -> Self {
        Self {
            driver,
            aggregator,
            running: false,
        }
    }

    /// Run the main collection and archiving loop
    pub async fn run(&mut self) -> Result<()> {
        self.running = true;

        info!("Scheduler started");
        info!("Archive interval: {}s", self.aggregator.interval());
        info!("Unit system: {}", self.aggregator.unit_system());

        while self.running {
            match self.process_packet().await {
                Ok(()) => {}
                Err(e) => {
                    error!("Error processing packet: {}", e);
                    // Continue running despite errors
                }
            }
        }

        info!("Scheduler stopped");
        Ok(())
    }

    /// Process a single packet cycle
    async fn process_packet(&mut self) -> Result<()> {
        // Get packet from driver
        let packet = self
            .driver
            .get_packet()
            .await
            .context("Failed to get packet from driver")?;

        info!(
            "Received packet: timestamp={}, observations={}",
            packet.date_time,
            packet.observations.len()
        );

        // Add to aggregator
        self.aggregator
            .add_packet(packet)
            .await
            .context("Failed to add packet to aggregator")?;

        Ok(())
    }

    /// Stop the scheduler and flush remaining data
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping scheduler...");
        self.running = false;

        // Stop driver
        if let Err(e) = self.driver.stop().await {
            warn!("Error stopping driver: {}", e);
        }

        // Flush any remaining buffered packets
        if let Err(e) = self.aggregator.force_flush().await {
            warn!("Error flushing aggregator: {}", e);
        }

        info!("Scheduler stopped successfully");
        Ok(())
    }

    /// Check if scheduler is running
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(test)]
mod tests {

    // Note: Full integration tests with DB are in tests/golden/
    // These are just structural tests

    #[test]
    fn test_scheduler_creation() {
        // Structural test - full tests require async runtime and DB
        // See golden tests for complete validation
    }
}
