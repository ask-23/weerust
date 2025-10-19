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
pub struct SinksConfig {
    pub http: Option<HttpSinkConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub station: Option<StationConfig>,
    pub sinks: Option<SinksConfig>,
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
