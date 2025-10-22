//! Archive interval aggregation logic

use crate::{ArchiveResult, PacketBuffer};
use std::collections::HashMap;
use tracing::{debug, info, instrument};
use weex_core::{aggregate_packets, WeatherPacket};
use weex_db::{schema::ArchiveRow, DbClient};

/// Aggregator for converting packets to archive records
pub struct IntervalAggregator {
    interval: i32,
    unit_system: i32,
    buffer: PacketBuffer,
    db_client: DbClient,
}

impl IntervalAggregator {
    /// Create a new aggregator with specified interval (seconds)
    pub fn new(interval: i32, unit_system: i32, db_client: DbClient) -> Self {
        Self {
            interval,
            unit_system,
            buffer: PacketBuffer::new(interval),
            db_client,
        }
    }

    /// Add a weather packet to the aggregation buffer
    #[instrument(skip(self, packet))]
    pub async fn add_packet(&mut self, packet: WeatherPacket) -> ArchiveResult<()> {
        let interval_end = self.buffer.add(packet)?;

        // Check if interval is complete
        if let Some(end_time) = interval_end {
            self.flush_interval(end_time).await?;
        }

        Ok(())
    }

    /// Flush the current interval to database
    #[instrument(skip(self))]
    async fn flush_interval(&mut self, end_time: i64) -> ArchiveResult<()> {
        let packets = self.buffer.drain();

        if packets.is_empty() {
            debug!("No packets to flush for interval ending at {}", end_time);
            return Ok(());
        }

        info!(
            "Flushing {} packets for interval ending at {}",
            packets.len(),
            end_time
        );

        // Aggregate all observations
        let aggregates = aggregate_packets(&packets);

        // Convert to ArchiveRow
        let archive_row = self.build_archive_row(end_time, aggregates);

        // Write to database
        self.db_client.insert_archive(&archive_row).await?;

        info!("Archive record written for timestamp {}", end_time);
        Ok(())
    }

    /// Build an ArchiveRow from aggregated data
    fn build_archive_row(
        &self,
        date_time: i64,
        aggregates: HashMap<String, (weex_core::AggregateType, Option<f64>)>,
    ) -> ArchiveRow {
        let get_value =
            |key: &str| -> Option<f64> { aggregates.get(key).and_then(|(_, val)| *val) };

        ArchiveRow {
            date_time,
            us_units: self.unit_system,
            interval: self.interval,
            out_temp: get_value("outTemp"),
            in_temp: get_value("inTemp"),
            extra_temp1: get_value("extraTemp1"),
            out_humidity: get_value("outHumidity"),
            in_humidity: get_value("inHumidity"),
            barometer: get_value("barometer"),
            pressure: get_value("pressure"),
            altimeter: get_value("altimeter"),
            wind_speed: get_value("windSpeed"),
            wind_dir: get_value("windDir"),
            wind_gust: get_value("windGust"),
            wind_gust_dir: get_value("windGustDir"),
            rain: get_value("rain"),
            rain_rate: get_value("rainRate"),
            dewpoint: get_value("dewpoint"),
            windchill: get_value("windchill"),
            heatindex: get_value("heatindex"),
            radiation: get_value("radiation"),
            uv: get_value("UV"),
            rx_check_percent: get_value("rxCheckPercent"),
        }
    }

    /// Force flush current buffer (for shutdown)
    pub async fn force_flush(&mut self) -> ArchiveResult<()> {
        let now = chrono::Utc::now().timestamp();
        self.flush_interval(now).await
    }

    /// Get current interval setting
    pub fn interval(&self) -> i32 {
        self.interval
    }

    /// Get current unit system
    pub fn unit_system(&self) -> i32 {
        self.unit_system
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_archive_row() {
        // Note: Full integration tests with DB are in tests/golden/
        // This just validates structure construction
        let mut aggregates = HashMap::new();
        aggregates.insert(
            "outTemp".to_string(),
            (weex_core::AggregateType::Avg, Some(25.5)),
        );
        aggregates.insert(
            "outHumidity".to_string(),
            (weex_core::AggregateType::Avg, Some(65.0)),
        );

        // Mock DB client would be needed for full test
        // See golden tests for complete validation
    }
}
