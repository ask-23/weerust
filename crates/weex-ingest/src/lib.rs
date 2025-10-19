//! Weather station driver adapters
//!
//! This crate provides the interface for receiving weather data from
//! various hardware stations. Drivers are currently stubbed for initial
//! development, to be implemented with actual hardware protocol support.

pub mod driver;
pub mod interceptor;
pub mod simulator;

pub use driver::*;
pub use interceptor::*;
pub use simulator::*;

use thiserror::Error;
use tokio::sync::mpsc;
use weex_core::WeatherPacket;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Invalid packet: {0}")]
    InvalidPacket(String),

    #[error("Timeout waiting for data")]
    Timeout,
}

pub type IngestResult<T> = Result<T, IngestError>;

/// Trait for all weather station drivers
#[async_trait::async_trait]
pub trait StationDriver: Send + Sync {
    /// Driver name/identifier
    fn name(&self) -> &str;

    /// Initialize the driver and start data collection
    async fn start(&mut self) -> IngestResult<()>;

    /// Stop the driver and clean up resources
    async fn stop(&mut self) -> IngestResult<()>;

    /// Get the next weather packet (blocking)
    async fn get_packet(&mut self) -> IngestResult<WeatherPacket>;

    /// Check if driver is currently active
    fn is_active(&self) -> bool;
}

/// Channel-based packet receiver for async communication
pub type PacketReceiver = mpsc::Receiver<WeatherPacket>;
pub type PacketSender = mpsc::Sender<WeatherPacket>;

/// Create a new packet channel with specified buffer size
pub fn create_packet_channel(buffer_size: usize) -> (PacketSender, PacketReceiver) {
    mpsc::channel(buffer_size)
}
