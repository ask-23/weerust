#![cfg(feature = "postgres")]
use anyhow::Result;
use sqlx::{Pool, Postgres};
use weex_core::{Sink, WeatherPacket};

pub struct PostgresSink {
    pool: Pool<Postgres>,
}

impl PostgresSink {
    pub async fn new(url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await?;
        // Minimal table: dt bigint, json text
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS packets (
                id SERIAL PRIMARY KEY,
                dt BIGINT NOT NULL,
                json TEXT NOT NULL
            );",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl Sink for PostgresSink {
    async fn emit(&mut self, packet: &WeatherPacket) -> Result<()> {
        let json = serde_json::to_string(packet)?;
        sqlx::query("INSERT INTO packets (dt, json) VALUES ($1, $2)")
            .bind(packet.date_time)
            .bind(json)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
