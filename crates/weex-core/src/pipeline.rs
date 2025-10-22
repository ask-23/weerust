use anyhow::Result;

use crate::WeatherPacket;

#[async_trait::async_trait]
pub trait Source: Send + Sync {
    async fn next_packet(&mut self) -> Result<WeatherPacket>;
}

#[async_trait::async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, packet: WeatherPacket) -> Result<WeatherPacket>;
}

#[async_trait::async_trait]
pub trait Sink: Send + Sync {
    async fn emit(&mut self, packet: &WeatherPacket) -> Result<()>;
}
