use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationConfig {
    pub id: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSinkConfig {
    pub bind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsSinkConfig {
    pub dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteSinkConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresSinkConfig {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluxSinkConfig {
    pub url: Option<String>,
    pub org: Option<String>,
    pub bucket: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinksConfig {
    pub http: Option<HttpSinkConfig>,
    pub fs: Option<FsSinkConfig>,
    pub sqlite: Option<SqliteSinkConfig>,
    pub postgres: Option<PostgresSinkConfig>,
    pub influx: Option<InfluxSinkConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptorConfig {
    pub bind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    pub interceptor: Option<InterceptorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub station: Option<StationConfig>,
    pub sinks: Option<SinksConfig>,
    pub ingest: Option<IngestConfig>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppConfig {
    /// Load configuration from WEEWX_CONFIG path (TOML) if present, with reasonable defaults
    pub fn load() -> Result<Self, ConfigError> {
        let path = std::env::var("WEEWX_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
        let cfg = if Path::new(&path).exists() {
            let s = fs::read_to_string(&path)?;
            toml::from_str::<AppConfig>(&s)?
        } else {
            AppConfig::default()
        };
        Ok(cfg)
    }

    /// Get HTTP bind address (default 0.0.0.0:8080)
    pub fn http_bind(&self) -> String {
        self.sinks
            .as_ref()
            .and_then(|s| s.http.as_ref())
            .and_then(|h| h.bind.clone())
            .unwrap_or_else(|| "0.0.0.0:8080".to_string())
    }

    /// Get INTERCEPTOR UDP bind address (default 0.0.0.0:9999)
    pub fn interceptor_bind(&self) -> String {
        self.ingest
            .as_ref()
            .and_then(|i| i.interceptor.as_ref())
            .and_then(|c| c.bind.clone())
            .unwrap_or_else(|| "0.0.0.0:9999".to_string())
    }

    /// Get filesystem sink directory if configured
    pub fn fs_dir(&self) -> Option<String> {
        self.sinks
            .as_ref()
            .and_then(|s| s.fs.as_ref())
            .and_then(|f| f.dir.clone())
    }

    /// Get SQLite sink path if configured
    pub fn sqlite_path(&self) -> Option<String> {
        self.sinks
            .as_ref()
            .and_then(|s| s.sqlite.as_ref())
            .and_then(|sq| sq.path.clone())
    }

    /// Get Postgres URL if configured
    pub fn postgres_url(&self) -> Option<String> {
        self.sinks
            .as_ref()
            .and_then(|s| s.postgres.as_ref())
            .and_then(|pg| pg.url.clone())
    }

    /// Get Influx configuration if configured
    pub fn influx_params(&self) -> Option<(String, String, String, String)> {
        let s = self.sinks.as_ref()?;
        let influx = s.influx.as_ref()?;
        let (url, org, bucket, token) = (
            influx.url.clone()?,
            influx.org.clone()?,
            influx.bucket.clone()?,
            influx.token.clone()?,
        );
        Some((url, org, bucket, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bind_is_8080() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.http_bind(), "0.0.0.0:8080");
    }
}
