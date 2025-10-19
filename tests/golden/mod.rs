//! Golden test harness for validating Rust implementation against Python WeeWX
//!
//! This harness:
//! 1. Loads captured packet JSON from fixtures
//! 2. Writes data to a test MySQL database clone
//! 3. Dumps the database state
//! 4. Compares against baseline dump from Python WeeWX
//!
//! Usage:
//! - Place captured packet JSON in tests/golden/fixtures/
//! - Place baseline DB dumps in tests/golden/baselines/
//! - Run: cargo test --test golden

pub mod db_diff;
pub mod fixtures;
pub mod test_db;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Golden test configuration
pub struct GoldenTestConfig {
    /// Path to fixture directory
    pub fixtures_dir: PathBuf,
    /// Path to baseline directory
    pub baselines_dir: PathBuf,
    /// Test database URL
    pub test_db_url: String,
    /// Whether to update baselines on mismatch
    pub update_baselines: bool,
}

impl GoldenTestConfig {
    pub fn default() -> Self {
        Self {
            fixtures_dir: PathBuf::from("tests/golden/fixtures"),
            baselines_dir: PathBuf::from("tests/golden/baselines"),
            test_db_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root@localhost/weewx_test".to_string()),
            update_baselines: std::env::var("UPDATE_BASELINES").is_ok(),
        }
    }

    pub fn fixture_path(&self, name: &str) -> PathBuf {
        self.fixtures_dir.join(format!("{}.json", name))
    }

    pub fn baseline_path(&self, name: &str) -> PathBuf {
        self.baselines_dir.join(format!("{}.sql", name))
    }
}

/// Result of a golden test run
#[derive(Debug)]
pub struct GoldenTestResult {
    pub test_name: String,
    pub passed: bool,
    pub differences: Vec<String>,
    pub actual_dump: String,
    pub expected_dump: String,
}

impl GoldenTestResult {
    pub fn assert_passed(&self) {
        if !self.passed {
            panic!(
                "Golden test '{}' failed with {} differences:\n{}",
                self.test_name,
                self.differences.len(),
                self.differences.join("\n")
            );
        }
    }
}
