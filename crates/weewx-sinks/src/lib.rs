use anyhow::Result;
#[cfg(feature = "influx")]
pub mod influx;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use weex_core::{Sink, WeatherPacket};

pub struct FsSink {
    _dir: PathBuf,
    file: PathBuf,
}

impl FsSink {
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        create_dir_all(&dir)?;
        let file = dir.join("packets.jsonl");
        Ok(Self { _dir: dir, file })
    }
}

#[async_trait::async_trait]
impl Sink for FsSink {
    async fn emit(&mut self, packet: &WeatherPacket) -> Result<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)?;
        let line = serde_json::to_string(packet)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn writes_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink = FsSink::new(dir.path()).unwrap();
        let mut obs = HashMap::new();
        obs.insert("outTemp".into(), weex_core::ObservationValue::Float(20.0));
        let pkt = WeatherPacket {
            date_time: 1,
            station: None,
            interval: Some(1),
            observations: obs,
        };
        sink.emit(&pkt).await.unwrap();
        let content = std::fs::read_to_string(dir.path().join("packets.jsonl")).unwrap();
        assert!(content.contains("outTemp"));
    }
}
