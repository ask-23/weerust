//! Aggregation and rollup calculations for archive intervals

use crate::types::{AggregateType, WeatherPacket};
use std::collections::HashMap;

/// Accumulator for calculating aggregates over multiple observations
#[derive(Debug, Clone)]
pub struct Accumulator {
    observations: Vec<f64>,
    aggregate_type: AggregateType,
}

impl Accumulator {
    pub fn new(aggregate_type: AggregateType) -> Self {
        Self {
            observations: Vec::new(),
            aggregate_type,
        }
    }

    pub fn add(&mut self, value: f64) {
        self.observations.push(value);
    }

    pub fn result(&self) -> Option<f64> {
        if self.observations.is_empty() {
            return None;
        }

        Some(match self.aggregate_type {
            AggregateType::Min => self
                .observations
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
            AggregateType::Max => self
                .observations
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
            AggregateType::Sum => self.observations.iter().sum(),
            AggregateType::Avg => {
                let sum: f64 = self.observations.iter().sum();
                sum / self.observations.len() as f64
            }
            AggregateType::Last => self.observations.last().copied()?,
            AggregateType::First => self.observations.first().copied()?,
            AggregateType::Count => self.observations.len() as f64,
        })
    }

    pub fn count(&self) -> usize {
        self.observations.len()
    }
}

/// Default aggregate type for common observation types
pub fn default_aggregate_type(obs_type: &str) -> AggregateType {
    match obs_type {
        "rain" => AggregateType::Sum,
        "outTemp" | "inTemp" | "dewpoint" | "heatindex" | "windchill" => AggregateType::Avg,
        "barometer" | "pressure" | "altimeter" => AggregateType::Avg,
        "windSpeed" => AggregateType::Avg,
        "windGust" => AggregateType::Max,
        "windDir" | "windGustDir" => AggregateType::Last,
        "outHumidity" | "inHumidity" => AggregateType::Avg,
        "radiation" => AggregateType::Avg,
        "UV" => AggregateType::Avg,
        _ => AggregateType::Last,
    }
}

/// Aggregate multiple weather packets into summary values
pub fn aggregate_packets(
    packets: &[WeatherPacket],
) -> HashMap<String, (AggregateType, Option<f64>)> {
    let mut accumulators: HashMap<String, Accumulator> = HashMap::new();

    for packet in packets {
        for (key, value) in &packet.observations {
            if let Some(numeric_value) = value.as_f64() {
                let aggregate_type = default_aggregate_type(key);
                accumulators
                    .entry(key.clone())
                    .or_insert_with(|| Accumulator::new(aggregate_type))
                    .add(numeric_value);
            }
        }
    }

    accumulators
        .into_iter()
        .map(|(key, acc)| {
            let agg_type = acc.aggregate_type;
            (key, (agg_type, acc.result()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulator_min() {
        let mut acc = Accumulator::new(AggregateType::Min);
        acc.add(10.0);
        acc.add(5.0);
        acc.add(15.0);
        assert_eq!(acc.result(), Some(5.0));
    }

    #[test]
    fn test_accumulator_max() {
        let mut acc = Accumulator::new(AggregateType::Max);
        acc.add(10.0);
        acc.add(5.0);
        acc.add(15.0);
        assert_eq!(acc.result(), Some(15.0));
    }

    #[test]
    fn test_accumulator_avg() {
        let mut acc = Accumulator::new(AggregateType::Avg);
        acc.add(10.0);
        acc.add(20.0);
        acc.add(30.0);
        assert_eq!(acc.result(), Some(20.0));
    }

    #[test]
    fn test_accumulator_sum() {
        let mut acc = Accumulator::new(AggregateType::Sum);
        acc.add(10.0);
        acc.add(20.0);
        acc.add(30.0);
        assert_eq!(acc.result(), Some(60.0));
    }

    #[test]
    fn test_accumulator_last() {
        let mut acc = Accumulator::new(AggregateType::Last);
        acc.add(10.0);
        acc.add(20.0);
        acc.add(30.0);
        assert_eq!(acc.result(), Some(30.0));
    }

    #[test]
    fn test_accumulator_empty() {
        let acc = Accumulator::new(AggregateType::Avg);
        assert_eq!(acc.result(), None);
    }

    #[test]
    fn test_default_aggregate_types() {
        assert_eq!(default_aggregate_type("rain"), AggregateType::Sum);
        assert_eq!(default_aggregate_type("outTemp"), AggregateType::Avg);
        assert_eq!(default_aggregate_type("windGust"), AggregateType::Max);
        assert_eq!(default_aggregate_type("windDir"), AggregateType::Last);
    }
}
