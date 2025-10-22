//! Database schema types matching Python WeeWX MySQL schema
//!
//! IMPORTANT: These structures must maintain strict parity with the
//! existing MySQL schema created by Python WeeWX. Do not modify
//! field names or types without verifying against production schema.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Archive table record (main weather data storage)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ArchiveRow {
    /// Primary timestamp (Unix epoch, end of interval)
    #[sqlx(rename = "dateTime")]
    pub date_time: i64,

    /// Unit system (1=US, 16=Metric, 17=MetricWX)
    #[sqlx(rename = "usUnits")]
    pub us_units: i32,

    /// Interval length in seconds
    pub interval: i32,

    // Temperature fields
    #[sqlx(rename = "outTemp")]
    pub out_temp: Option<f64>,

    #[sqlx(rename = "inTemp")]
    pub in_temp: Option<f64>,

    #[sqlx(rename = "extraTemp1")]
    pub extra_temp1: Option<f64>,

    // Humidity fields
    #[sqlx(rename = "outHumidity")]
    pub out_humidity: Option<f64>,

    #[sqlx(rename = "inHumidity")]
    pub in_humidity: Option<f64>,

    // Pressure fields
    pub barometer: Option<f64>,
    pub pressure: Option<f64>,
    pub altimeter: Option<f64>,

    // Wind fields
    #[sqlx(rename = "windSpeed")]
    pub wind_speed: Option<f64>,

    #[sqlx(rename = "windDir")]
    pub wind_dir: Option<f64>,

    #[sqlx(rename = "windGust")]
    pub wind_gust: Option<f64>,

    #[sqlx(rename = "windGustDir")]
    pub wind_gust_dir: Option<f64>,

    // Rain fields
    pub rain: Option<f64>,

    #[sqlx(rename = "rainRate")]
    pub rain_rate: Option<f64>,

    // Derived fields
    pub dewpoint: Option<f64>,
    pub windchill: Option<f64>,
    pub heatindex: Option<f64>,

    // Solar fields
    pub radiation: Option<f64>,

    #[sqlx(rename = "UV")]
    pub uv: Option<f64>,

    // Extra fields
    #[sqlx(rename = "rxCheckPercent")]
    pub rx_check_percent: Option<f64>,
}

/// Metadata table for storing configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MetadataRow {
    pub name: String,
    pub value: String,
}

/// Daily summary statistics (optional table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DailySummaryRow {
    #[sqlx(rename = "dateTime")]
    pub date_time: i64,

    pub obs_type: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub sum: Option<f64>,
    pub count: i32,
}

/// Table names matching Python WeeWX schema
pub mod tables {
    pub const ARCHIVE: &str = "archive";
    pub const METADATA: &str = "archive_metadata";
    pub const DAILY_SUMMARY: &str = "archive_day_summary";
}

/// Expected database version (must match Python WeeWX)
pub const EXPECTED_SCHEMA_VERSION: &str = "4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_row_size() {
        // Ensure structure size is reasonable
        use std::mem::size_of;
        let size = size_of::<ArchiveRow>();
        assert!(size > 0);
        assert!(size < 1024); // Sanity check
    }

    #[test]
    fn test_table_names() {
        assert_eq!(tables::ARCHIVE, "archive");
        assert_eq!(tables::METADATA, "archive_metadata");
    }
}
