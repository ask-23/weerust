//! Database query operations for WeeWX tables

use crate::schema::{ArchiveRow, MetadataRow};
use crate::{DbClient, DbError, DbResult};
use sqlx::Row;
use tracing::{debug, instrument};

impl DbClient {
    /// Insert a single archive record
    #[instrument(skip(self, record))]
    pub async fn insert_archive(&self, record: &ArchiveRow) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO archive (
                dateTime, usUnits, interval,
                outTemp, inTemp, extraTemp1,
                outHumidity, inHumidity,
                barometer, pressure, altimeter,
                windSpeed, windDir, windGust, windGustDir,
                rain, rainRate,
                dewpoint, windchill, heatindex,
                radiation, UV, rxCheckPercent
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.date_time)
        .bind(record.us_units)
        .bind(record.interval)
        .bind(record.out_temp)
        .bind(record.in_temp)
        .bind(record.extra_temp1)
        .bind(record.out_humidity)
        .bind(record.in_humidity)
        .bind(record.barometer)
        .bind(record.pressure)
        .bind(record.altimeter)
        .bind(record.wind_speed)
        .bind(record.wind_dir)
        .bind(record.wind_gust)
        .bind(record.wind_gust_dir)
        .bind(record.rain)
        .bind(record.rain_rate)
        .bind(record.dewpoint)
        .bind(record.windchill)
        .bind(record.heatindex)
        .bind(record.radiation)
        .bind(record.uv)
        .bind(record.rx_check_percent)
        .execute(self.pool())
        .await?;

        debug!("Inserted archive record for timestamp {}", record.date_time);
        Ok(())
    }

    /// Get archive records within a time range
    #[instrument(skip(self))]
    pub async fn get_archive_range(
        &self,
        start_time: i64,
        end_time: i64,
    ) -> DbResult<Vec<ArchiveRow>> {
        let records = sqlx::query_as::<_, ArchiveRow>(
            r#"
            SELECT * FROM archive
            WHERE dateTime >= ? AND dateTime <= ?
            ORDER BY dateTime ASC
            "#,
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(self.pool())
        .await?;

        debug!(
            "Retrieved {} archive records between {} and {}",
            records.len(),
            start_time,
            end_time
        );
        Ok(records)
    }

    /// Get the most recent archive record
    #[instrument(skip(self))]
    pub async fn get_latest_archive(&self) -> DbResult<Option<ArchiveRow>> {
        let record = sqlx::query_as::<_, ArchiveRow>(
            r#"
            SELECT * FROM archive
            ORDER BY dateTime DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(record)
    }

    /// Get metadata value by name
    #[instrument(skip(self))]
    pub async fn get_metadata(&self, name: &str) -> DbResult<Option<String>> {
        let row = sqlx::query(
            r#"
            SELECT value FROM archive_metadata WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(|r| r.get("value")))
    }

    /// Set metadata value
    #[instrument(skip(self))]
    pub async fn set_metadata(&self, name: &str, value: &str) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO archive_metadata (name, value)
            VALUES (?, ?)
            ON DUPLICATE KEY UPDATE value = VALUES(value)
            "#,
        )
        .bind(name)
        .bind(value)
        .execute(self.pool())
        .await?;

        debug!("Set metadata: {} = {}", name, value);
        Ok(())
    }

    /// Get count of archive records
    #[instrument(skip(self))]
    pub async fn count_archive_records(&self) -> DbResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM archive")
            .fetch_one(self.pool())
            .await?;

        Ok(row.get("count"))
    }

    /// Delete archive records older than timestamp
    #[instrument(skip(self))]
    pub async fn delete_archive_before(&self, timestamp: i64) -> DbResult<u64> {
        let result = sqlx::query("DELETE FROM archive WHERE dateTime < ?")
            .bind(timestamp)
            .execute(self.pool())
            .await?;

        let deleted = result.rows_affected();
        debug!("Deleted {} archive records before {}", deleted, timestamp);
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests with real database are in tests/golden/
    // These are just unit tests for query structure validation

    #[test]
    fn test_query_syntax() {
        // Queries are validated at runtime by sqlx
        // This test just ensures module compiles
        assert!(true);
    }
}
