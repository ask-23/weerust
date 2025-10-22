use anyhow::Result;
use rusqlite::{params, Connection};
use weex_core::WeatherPacket;

pub struct SqliteSink {
    conn: Connection,
}

impl SqliteSink {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS packets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dt INTEGER NOT NULL,
                json TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn emit_sync(&mut self, packet: &WeatherPacket) -> Result<()> {
        let json = serde_json::to_string(packet)?;
        self.conn.execute(
            "INSERT INTO packets (dt, json) VALUES (?1, ?2)",
            params![packet.date_time, json],
        )?;
        Ok(())
    }
}

// NOTE: Cannot implement Sink trait because rusqlite::Connection is not Sync
// TODO: Use tokio_rusqlite or Arc<Mutex<Connection>> for async support

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_packet() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("weewx.db");
        let mut sink = SqliteSink::new(&db_path).unwrap();
        let pkt = weex_core::WeatherPacket {
            date_time: 1,
            station: None,
            interval: None,
            observations: Default::default(),
        };
        sink.emit_sync(&pkt).unwrap();
        let count: i64 = sink
            .conn
            .query_row("SELECT COUNT(*) FROM packets", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
