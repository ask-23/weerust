//! Simulated weather station for testing

use crate::{IngestError, IngestResult, StationDriver};
use weex_core::{unit_systems, ObservationValue, WeatherPacket};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

/// Simulator driver that generates synthetic weather data
pub struct SimulatorDriver {
    interval: u64,
    active: bool,
    base_temp: f64,
}

impl SimulatorDriver {
    /// Create a new simulator with specified interval (seconds)
    pub fn new(interval: u64) -> Self {
        Self {
            interval,
            active: false,
            base_temp: 20.0, // 20°C base temperature
        }
    }

    fn generate_packet(&mut self) -> WeatherPacket {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Add some pseudo-random variation
        let variation = ((now % 100) as f64 / 10.0) - 5.0;
        let temp = self.base_temp + variation;

        let mut observations = HashMap::new();
        observations.insert(
            "outTemp".to_string(),
            ObservationValue::Float(temp),
        );
        observations.insert(
            "outHumidity".to_string(),
            ObservationValue::Float(65.0 + variation),
        );
        observations.insert(
            "barometer".to_string(),
            ObservationValue::Float(1013.25 + variation * 2.0),
        );
        observations.insert(
            "windSpeed".to_string(),
            ObservationValue::Float(5.0 + variation.abs()),
        );
        observations.insert(
            "windDir".to_string(),
            ObservationValue::Float((now % 360) as f64),
        );
        observations.insert(
            "rain".to_string(),
            ObservationValue::Float(0.0),
        );

        WeatherPacket {
            date_time: now,
            station: Some("simulator".to_string()),
            interval: Some(self.interval as i32),
            observations,
        }
    }
}

#[async_trait::async_trait]
impl StationDriver for SimulatorDriver {
    fn name(&self) -> &str {
        "simulator"
    }

    async fn start(&mut self) -> IngestResult<()> {
        if self.active {
            return Err(IngestError::DriverError(
                "Driver already started".to_string(),
            ));
        }
        self.active = true;
        tracing::info!("Simulator driver started with {}s interval", self.interval);
        Ok(())
    }

    async fn stop(&mut self) -> IngestResult<()> {
        if !self.active {
            return Err(IngestError::DriverError(
                "Driver not started".to_string(),
            ));
        }
        self.active = false;
        tracing::info!("Simulator driver stopped");
        Ok(())
    }

    async fn get_packet(&mut self) -> IngestResult<WeatherPacket> {
        if !self.active {
            return Err(IngestError::DriverError("Driver not active".to_string()));
        }

        // Simulate interval delay
        sleep(Duration::from_secs(self.interval)).await;

        Ok(self.generate_packet())
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulator_lifecycle() {
        let mut driver = SimulatorDriver::new(1);

        assert!(!driver.is_active());

        driver.start().await.unwrap();
        assert!(driver.is_active());

        // Start again should fail
        assert!(driver.start().await.is_err());

        driver.stop().await.unwrap();
        assert!(!driver.is_active());
    }

    #[tokio::test]
    async fn test_simulator_packet_generation() {
        let mut driver = SimulatorDriver::new(0); // No delay for testing
        driver.start().await.unwrap();

        let packet = driver.generate_packet();

        assert!(packet.date_time > 0);
        assert_eq!(packet.station, Some("simulator".to_string()));
        assert!(packet.observations.contains_key("outTemp"));
        assert!(packet.observations.contains_key("outHumidity"));
        assert!(packet.observations.contains_key("barometer"));
    }
}
