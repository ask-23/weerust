//! Test database management for golden tests

use anyhow::{Context, Result};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::Row;
use std::time::Duration;

/// Test database manager
pub struct TestDb {
    pool: MySqlPool,
    db_name: String,
}

impl TestDb {
    /// Create a new test database instance
    pub async fn new(base_url: &str, test_name: &str) -> Result<Self> {
        let db_name = format!("weewx_test_{}", test_name.replace('-', "_"));

        // Connect to MySQL without database
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect(base_url)
            .await
            .context("Failed to connect to MySQL")?;

        // Drop existing test database if it exists
        sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
            .execute(&pool)
            .await
            .context("Failed to drop test database")?;

        // Create fresh test database
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&pool)
            .await
            .context("Failed to create test database")?;

        pool.close().await;

        // Reconnect to the new database
        let db_url = format!("{}/{}", base_url.trim_end_matches('/'), db_name);
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&db_url)
            .await
            .context("Failed to connect to test database")?;

        Ok(Self { pool, db_name })
    }

    /// Get database URL
    pub fn url(&self) -> String {
        format!("mysql://localhost/{}", self.db_name)
    }

    /// Get pool reference
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// Initialize schema from SQL file
    pub async fn init_schema(&self, schema_sql: &str) -> Result<()> {
        // Split into individual statements and execute
        for statement in schema_sql.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() && !statement.starts_with("--") {
                sqlx::query(statement)
                    .execute(&self.pool)
                    .await
                    .with_context(|| format!("Failed to execute: {}", statement))?;
            }
        }

        Ok(())
    }

    /// Clear all data from tables (keep schema)
    pub async fn clear_data(&self) -> Result<()> {
        sqlx::query("DELETE FROM archive")
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM archive_metadata")
            .execute(&self.pool)
            .await
            .ok(); // May not exist

        Ok(())
    }

    /// Get row count for a table
    pub async fn count_rows(&self, table: &str) -> Result<i64> {
        let row = sqlx::query(&format!("SELECT COUNT(*) as count FROM {}", table))
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    /// Verify database is ready
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Note: Cannot do async cleanup in Drop
        // Test databases should be cleaned up manually or by CI
    }
}

/// Load the standard WeeWX schema
pub fn weewx_schema() -> &'static str {
    r#"
    CREATE TABLE archive (
        dateTime INT NOT NULL PRIMARY KEY,
        usUnits INT NOT NULL,
        `interval` INT NOT NULL,
        outTemp REAL,
        inTemp REAL,
        extraTemp1 REAL,
        outHumidity REAL,
        inHumidity REAL,
        barometer REAL,
        pressure REAL,
        altimeter REAL,
        windSpeed REAL,
        windDir REAL,
        windGust REAL,
        windGustDir REAL,
        rain REAL,
        rainRate REAL,
        dewpoint REAL,
        windchill REAL,
        heatindex REAL,
        radiation REAL,
        UV REAL,
        rxCheckPercent REAL
    );

    CREATE TABLE archive_metadata (
        name VARCHAR(255) NOT NULL PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires MySQL server
    async fn test_db_creation() {
        let base_url = "mysql://root@localhost";
        let test_db = TestDb::new(base_url, "test_creation").await.unwrap();

        // Verify connection
        test_db.ping().await.unwrap();

        // Initialize schema
        test_db.init_schema(weewx_schema()).await.unwrap();

        // Verify table exists
        let count = test_db.count_rows("archive").await.unwrap();
        assert_eq!(count, 0);
    }
}
