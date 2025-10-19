use anyhow::{Context, Result};
use glob::glob;
use mysql::prelude::*;
use mysql::{OptsBuilder, Pool};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
struct WeatherPacket {
    timestamp: i64,
    temperature: Option<f64>,
    humidity: Option<f64>,
    pressure: Option<f64>,
    wind_speed: Option<f64>,
    wind_direction: Option<f64>,
    rain: Option<f64>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

struct GoldenTestRunner {
    pool: Pool,
    test_db_name: String,
}

impl GoldenTestRunner {
    fn new(database_url: &str, test_db_name: &str) -> Result<Self> {
        let opts = OptsBuilder::from_opts(
            mysql::Opts::from_url(database_url)
                .context("Failed to parse database URL")?,
        );
        let pool = Pool::new(opts).context("Failed to create connection pool")?;

        Ok(Self {
            pool,
            test_db_name: test_db_name.to_string(),
        })
    }

    fn setup_test_database(&self) -> Result<()> {
        let mut conn = self.pool.get_conn()?;

        // Drop and recreate test database
        conn.query_drop(format!("DROP DATABASE IF EXISTS {}", self.test_db_name))?;
        conn.query_drop(format!("CREATE DATABASE {}", self.test_db_name))?;
        conn.query_drop(format!("USE {}", self.test_db_name))?;

        // Create WeeWX-compatible schema
        conn.query_drop(
            r"CREATE TABLE archive (
                dateTime INTEGER NOT NULL UNIQUE PRIMARY KEY,
                usUnits INTEGER NOT NULL,
                `interval` INTEGER NOT NULL,
                barometer REAL,
                pressure REAL,
                altimeter REAL,
                inTemp REAL,
                outTemp REAL,
                inHumidity REAL,
                outHumidity REAL,
                windSpeed REAL,
                windDir REAL,
                windGust REAL,
                windGustDir REAL,
                rainRate REAL,
                rain REAL,
                dewpoint REAL,
                windchill REAL,
                heatindex REAL,
                ET REAL,
                radiation REAL,
                UV REAL
            )"
        )?;

        println!("✅ Test database '{}' created successfully", self.test_db_name);
        Ok(())
    }

    fn load_packets(&self, packets_dir: &Path) -> Result<Vec<WeatherPacket>> {
        let pattern = packets_dir.join("*.json");
        let mut packets = Vec::new();

        for entry in glob(pattern.to_str().unwrap())? {
            let path = entry?;
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;

            let packet: WeatherPacket = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse {}", path.display()))?;

            packets.push(packet);
            println!("📦 Loaded packet: {}", path.display());
        }

        packets.sort_by_key(|p| p.timestamp);
        println!("✅ Loaded {} packets", packets.len());
        Ok(packets)
    }

    fn write_packets(&self, packets: &[WeatherPacket]) -> Result<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop(format!("USE {}", self.test_db_name))?;

        for packet in packets {
            conn.exec_drop(
                r"INSERT INTO archive (
                    dateTime, usUnits, `interval`,
                    outTemp, outHumidity, pressure,
                    windSpeed, windDir, rain
                ) VALUES (?, 1, 5, ?, ?, ?, ?, ?, ?)",
                (
                    packet.timestamp,
                    packet.temperature,
                    packet.humidity,
                    packet.pressure,
                    packet.wind_speed,
                    packet.wind_direction,
                    packet.rain,
                ),
            )?;
        }

        println!("✅ Wrote {} packets to database", packets.len());
        Ok(())
    }

    fn export_database(&self, output_path: &Path) -> Result<()> {
        let output = Command::new("mysqldump")
            .args([
                "--skip-comments",
                "--compact",
                "--skip-extended-insert",
                &self.test_db_name,
            ])
            .output()
            .context("Failed to run mysqldump")?;

        if !output.status.success() {
            anyhow::bail!(
                "mysqldump failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        fs::write(output_path, &output.stdout)
            .with_context(|| format!("Failed to write dump to {}", output_path.display()))?;

        println!("✅ Exported database to {}", output_path.display());
        Ok(())
    }

    fn diff_databases(&self, baseline_path: &Path, actual_path: &Path) -> Result<bool> {
        let baseline = fs::read_to_string(baseline_path)
            .context("Failed to read baseline dump")?;
        let actual = fs::read_to_string(actual_path)
            .context("Failed to read actual dump")?;

        // Normalize dumps (remove timestamps, AUTO_INCREMENT values, etc.)
        let baseline_normalized = self.normalize_dump(&baseline);
        let actual_normalized = self.normalize_dump(&actual);

        if baseline_normalized == actual_normalized {
            println!("✅ Database dumps match baseline");
            Ok(true)
        } else {
            println!("❌ Database dumps differ from baseline");

            // Write diff file for inspection
            let diff_output = Command::new("diff")
                .args(["-u", baseline_path.to_str().unwrap(), actual_path.to_str().unwrap()])
                .output();

            if let Ok(diff) = diff_output {
                let diff_path = actual_path.with_extension("diff");
                fs::write(&diff_path, &diff.stdout)?;
                println!("📝 Diff written to {}", diff_path.display());
            }

            Ok(false)
        }
    }

    fn normalize_dump(&self, dump: &str) -> String {
        dump.lines()
            .filter(|line| {
                // Skip comments and variable settings
                !line.starts_with("--") &&
                !line.starts_with("/*") &&
                !line.contains("AUTO_INCREMENT=")
            })
            .map(|line| {
                // Normalize whitespace
                line.split_whitespace().collect::<Vec<_>>().join(" ")
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn cleanup(&self) -> Result<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop(format!("DROP DATABASE IF EXISTS {}", self.test_db_name))?;
        println!("🧹 Cleaned up test database");
        Ok(())
    }
}

fn main() -> Result<()> {
    println!("🧪 Golden Test Runner\n");

    // Configuration
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root@localhost:3306".to_string());
    let test_db_name = "weewx_golden_test";
    let packets_dir = Path::new("../packets");
    let baseline_dir = Path::new("../baseline");
    let output_dump = Path::new("../baseline/actual_dump.sql");

    // Initialize test runner
    let runner = GoldenTestRunner::new(&database_url, test_db_name)?;

    // Run tests
    println!("1️⃣  Setting up test database...");
    runner.setup_test_database()?;

    println!("\n2️⃣  Loading JSON packets...");
    let packets = runner.load_packets(packets_dir)?;

    println!("\n3️⃣  Writing packets to database...");
    runner.write_packets(&packets)?;

    println!("\n4️⃣  Exporting database dump...");
    runner.export_database(output_dump)?;

    println!("\n5️⃣  Comparing with baseline...");
    let baseline_dump = baseline_dir.join("expected_dump.sql");

    if baseline_dump.exists() {
        let matches = runner.diff_databases(&baseline_dump, output_dump)?;

        if !matches {
            println!("\n⚠️  To update baseline:");
            println!("   cp {} {}",
                output_dump.display(),
                baseline_dump.display()
            );
            std::process::exit(1);
        }
    } else {
        println!("⚠️  No baseline found at {}", baseline_dump.display());
        println!("   Creating initial baseline...");
        fs::create_dir_all(baseline_dir)?;
        fs::copy(output_dump, &baseline_dump)?;
        println!("✅ Baseline created");
    }

    println!("\n6️⃣  Cleaning up...");
    runner.cleanup()?;

    println!("\n✨ All golden tests passed!");
    Ok(())
}
