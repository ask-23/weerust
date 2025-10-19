//! Driver registry and management

use crate::{IngestError, IngestResult, StationDriver};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for available station drivers
pub struct DriverRegistry {
    drivers: Arc<RwLock<HashMap<String, Box<dyn DriverFactory>>>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self {
            drivers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new driver factory
    pub async fn register<F>(&self, name: String, factory: F)
    where
        F: DriverFactory + 'static,
    {
        let mut drivers = self.drivers.write().await;
        drivers.insert(name, Box::new(factory));
    }

    /// Create a driver instance by name
    pub async fn create(&self, name: &str) -> IngestResult<Box<dyn StationDriver>> {
        let drivers = self.drivers.read().await;
        let factory = drivers
            .get(name)
            .ok_or_else(|| IngestError::DriverError(format!("Unknown driver: {}", name)))?;
        factory.create()
    }

    /// List all available driver names
    pub async fn list_drivers(&self) -> Vec<String> {
        let drivers = self.drivers.read().await;
        drivers.keys().cloned().collect()
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory trait for creating driver instances
pub trait DriverFactory: Send + Sync {
    fn create(&self) -> IngestResult<Box<dyn StationDriver>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::SimulatorDriver;

    struct TestDriverFactory;

    impl DriverFactory for TestDriverFactory {
        fn create(&self) -> IngestResult<Box<dyn StationDriver>> {
            Ok(Box::new(SimulatorDriver::new(300)))
        }
    }

    #[tokio::test]
    async fn test_driver_registry() {
        let registry = DriverRegistry::new();
        registry
            .register("simulator".to_string(), TestDriverFactory)
            .await;

        let drivers = registry.list_drivers().await;
        assert!(drivers.contains(&"simulator".to_string()));

        let driver = registry.create("simulator").await.unwrap();
        assert_eq!(driver.name(), "simulator");
    }
}
