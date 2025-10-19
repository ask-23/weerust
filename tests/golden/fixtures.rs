//! Fixture loading for golden tests

use anyhow::{Context, Result};
use serde_json;
use std::fs;
use std::path::Path;
use weex_core::WeatherPacket;

/// Load a single packet fixture
pub fn load_packet(path: &Path) -> Result<WeatherPacket> {
    let json = fs::read_to_string(path)
        .with_context(|| format!("Failed to read fixture: {:?}", path))?;

    serde_json::from_str(&json)
        .with_context(|| format!("Failed to parse packet JSON: {:?}", path))
}

/// Load multiple packets from a fixture file
pub fn load_packets(path: &Path) -> Result<Vec<WeatherPacket>> {
    let json = fs::read_to_string(path)
        .with_context(|| format!("Failed to read fixture: {:?}", path))?;

    serde_json::from_str(&json)
        .with_context(|| format!("Failed to parse packets JSON: {:?}", path))
}

/// Load all fixtures from a directory
pub fn load_all_fixtures(dir: &Path) -> Result<Vec<(String, Vec<WeatherPacket>)>> {
    let mut fixtures = Vec::new();

    if !dir.exists() {
        fs::create_dir_all(dir)?;
        return Ok(fixtures);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            let packets = load_packets(&path)?;
            fixtures.push((name, packets));
        }
    }

    Ok(fixtures)
}

/// Save packets to a fixture file (for creating test data)
pub fn save_packets(path: &Path, packets: &[WeatherPacket]) -> Result<()> {
    let json = serde_json::to_string_pretty(packets)?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write fixture: {:?}", path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use weex_core::ObservationValue;
    use tempfile::TempDir;

    fn make_test_packet() -> WeatherPacket {
        let mut observations = HashMap::new();
        observations.insert("outTemp".to_string(), ObservationValue::Float(25.5));
        observations.insert("outHumidity".to_string(), ObservationValue::Float(65.0));

        WeatherPacket {
            date_time: 1234567890,
            station: Some("test".to_string()),
            interval: Some(300),
            observations,
        }
    }

    #[test]
    fn test_save_and_load_packets() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("test.json");

        let packets = vec![make_test_packet()];

        // Save
        save_packets(&fixture_path, &packets).unwrap();

        // Load
        let loaded = load_packets(&fixture_path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].date_time, 1234567890);
    }

    #[test]
    fn test_load_all_fixtures() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        // Create test fixtures
        let packets = vec![make_test_packet()];
        save_packets(&dir.join("test1.json"), &packets).unwrap();
        save_packets(&dir.join("test2.json"), &packets).unwrap();

        // Load all
        let fixtures = load_all_fixtures(dir).unwrap();

        assert_eq!(fixtures.len(), 2);
    }
}
