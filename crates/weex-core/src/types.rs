//! Core data types for weather observations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Timestamp type (Unix epoch seconds)
pub type Timestamp = i64;

/// Observation interval in seconds
pub type Interval = i32;

/// Weather data packet from a station
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeatherPacket {
    /// Unix timestamp of observation
    #[serde(rename = "dateTime")]
    pub date_time: Timestamp,

    /// Station identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station: Option<String>,

    /// Observation interval (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<Interval>,

    /// Weather observations (field name -> value)
    #[serde(flatten)]
    pub observations: HashMap<String, ObservationValue>,
}

/// An observation value with optional null handling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ObservationValue {
    Float(f64),
    Integer(i64),
    String(String),
    Null,
}

impl ObservationValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ObservationValue::Float(v) => Some(*v),
            ObservationValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ObservationValue::Integer(v) => Some(*v),
            ObservationValue::Float(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, ObservationValue::Null)
    }
}

/// Archive record with aggregated data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchiveRecord {
    /// Unix timestamp (end of interval)
    #[serde(rename = "dateTime")]
    pub date_time: Timestamp,

    /// Observation interval (seconds)
    pub interval: Interval,

    /// Unit system (1=US, 16=Metric, 17=MetricWX)
    #[serde(rename = "usUnits")]
    pub us_units: i32,

    /// Aggregated observations
    #[serde(flatten)]
    pub aggregates: HashMap<String, ObservationValue>,
}

/// Aggregation type for rollups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregateType {
    Min,
    Max,
    Sum,
    Avg,
    Last,
    First,
    Count,
}

/// Unit system constants (must match Python WeeWX)
pub mod unit_systems {
    pub const US: i32 = 1;
    pub const METRIC: i32 = 16;
    pub const METRICWX: i32 = 17;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_value_conversions() {
        let float_val = ObservationValue::Float(25.5);
        assert_eq!(float_val.as_f64(), Some(25.5));

        let int_val = ObservationValue::Integer(42);
        assert_eq!(int_val.as_i64(), Some(42));
        assert_eq!(int_val.as_f64(), Some(42.0));

        let null_val = ObservationValue::Null;
        assert!(null_val.is_null());
        assert_eq!(null_val.as_f64(), None);
    }

    #[test]
    fn test_weather_packet_serde() {
        let json = r#"{"dateTime":1234567890,"outTemp":25.5,"interval":300}"#;
        let packet: WeatherPacket = serde_json::from_str(json).unwrap();

        assert_eq!(packet.date_time, 1234567890);
        assert_eq!(packet.interval, Some(300));
    }
}
