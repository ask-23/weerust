//! Unit conversion utilities
//!
//! Maintains parity with Python WeeWX unit system and conversions

use crate::types::unit_systems;

/// Unit conversion error
#[derive(Debug, thiserror::Error)]
pub enum UnitError {
    #[error("Unknown unit system: {0}")]
    UnknownUnitSystem(i32),

    #[error("Unknown observation type: {0}")]
    UnknownObservationType(String),

    #[error("Conversion not supported")]
    ConversionNotSupported,
}

/// Unit group for observation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitGroup {
    Temperature,
    Pressure,
    Rain,
    RainRate,
    Speed,
    Direction,
    Humidity,
    Radiation,
    Count,
}

/// Get unit group for an observation type
pub fn get_unit_group(obs_type: &str) -> Option<UnitGroup> {
    match obs_type {
        "outTemp" | "inTemp" | "dewpoint" | "heatindex" | "windchill" => {
            Some(UnitGroup::Temperature)
        }
        "barometer" | "pressure" | "altimeter" => Some(UnitGroup::Pressure),
        "rain" => Some(UnitGroup::Rain),
        "rainRate" => Some(UnitGroup::RainRate),
        "windSpeed" | "windGust" => Some(UnitGroup::Speed),
        "windDir" | "windGustDir" => Some(UnitGroup::Direction),
        "outHumidity" | "inHumidity" => Some(UnitGroup::Humidity),
        "radiation" => Some(UnitGroup::Radiation),
        _ => None,
    }
}

/// Convert value between unit systems
pub fn convert(
    value: f64,
    from_unit: i32,
    to_unit: i32,
    unit_group: UnitGroup,
) -> Result<f64, UnitError> {
    if from_unit == to_unit {
        return Ok(value);
    }

    match (from_unit, to_unit, unit_group) {
        // US to Metric temperature (F to C)
        (unit_systems::US, unit_systems::METRIC, UnitGroup::Temperature) => {
            Ok((value - 32.0) * 5.0 / 9.0)
        }
        // Metric to US temperature (C to F)
        (unit_systems::METRIC, unit_systems::US, UnitGroup::Temperature) => {
            Ok(value * 9.0 / 5.0 + 32.0)
        }
        // US to Metric pressure (inHg to mbar)
        (unit_systems::US, unit_systems::METRIC, UnitGroup::Pressure) => Ok(value * 33.8639),
        // Metric to US pressure (mbar to inHg)
        (unit_systems::METRIC, unit_systems::US, UnitGroup::Pressure) => Ok(value / 33.8639),
        // US to Metric rain (in to cm)
        (unit_systems::US, unit_systems::METRIC, UnitGroup::Rain) => Ok(value * 2.54),
        // Metric to US rain (cm to in)
        (unit_systems::METRIC, unit_systems::US, UnitGroup::Rain) => Ok(value / 2.54),
        // US to Metric speed (mph to kph)
        (unit_systems::US, unit_systems::METRIC, UnitGroup::Speed) => Ok(value * 1.60934),
        // Metric to US speed (kph to mph)
        (unit_systems::METRIC, unit_systems::US, UnitGroup::Speed) => Ok(value / 1.60934),
        _ => Err(UnitError::ConversionNotSupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_conversion() {
        // F to C: 32F = 0C
        let result = convert(
            32.0,
            unit_systems::US,
            unit_systems::METRIC,
            UnitGroup::Temperature,
        )
        .unwrap();
        assert!((result - 0.0).abs() < 0.001);

        // C to F: 0C = 32F
        let result = convert(
            0.0,
            unit_systems::METRIC,
            unit_systems::US,
            UnitGroup::Temperature,
        )
        .unwrap();
        assert!((result - 32.0).abs() < 0.001);

        // F to C: 212F = 100C
        let result = convert(
            212.0,
            unit_systems::US,
            unit_systems::METRIC,
            UnitGroup::Temperature,
        )
        .unwrap();
        assert!((result - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_same_unit_conversion() {
        let result = convert(
            25.0,
            unit_systems::US,
            unit_systems::US,
            UnitGroup::Temperature,
        )
        .unwrap();
        assert_eq!(result, 25.0);
    }

    #[test]
    fn test_unit_group_detection() {
        assert_eq!(get_unit_group("outTemp"), Some(UnitGroup::Temperature));
        assert_eq!(get_unit_group("barometer"), Some(UnitGroup::Pressure));
        assert_eq!(get_unit_group("windSpeed"), Some(UnitGroup::Speed));
        assert_eq!(get_unit_group("unknown"), None);
    }
}
