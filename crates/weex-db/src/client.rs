//! Database client and connection management

use crate::{DbError, DbResult};
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};
use sqlx::ConnectOptions;
use std::time::Duration;

/// Database client wrapping sqlx connection pool
#[derive(Clone)]
pub struct DbClient {
    pool: MySqlPool,
}

impl DbClient {
    /// Create a new database client from connection string
    pub async fn new(database_url: &str) -> DbResult<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Create a new database client with custom options
    pub async fn with_options(opts: MySqlConnectOptions) -> DbResult<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    /// Get reference to underlying pool for direct queries
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// Test the database connection
    pub async fn ping(&self) -> DbResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    /// Close the connection pool gracefully
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Build MySQL connection options from components
pub struct DbConnectionBuilder {
    host: String,
    port: u16,
    database: String,
    username: String,
    password: Option<String>,
}

impl DbConnectionBuilder {
    pub fn new(database: impl Into<String>) -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3306,
            database: database.into(),
            username: "weewx".to_string(),
            password: None,
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn build(self) -> MySqlConnectOptions {
        let mut opts = MySqlConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .database(&self.database)
            .username(&self.username);

        if let Some(password) = self.password {
            opts = opts.password(&password);
        }

        opts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_builder() {
        let opts = DbConnectionBuilder::new("weewx")
            .host("db.example.com")
            .port(3307)
            .username("admin")
            .password("secret")
            .build();

        // Just verify it builds without panicking
        // Actual connection tests require a real database
    }
}
