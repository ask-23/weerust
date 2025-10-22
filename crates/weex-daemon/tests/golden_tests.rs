//! Golden tests - Integration tests comparing Rust vs Python WeeWX output
#![cfg(feature = "legacy_golden")]
// LEGACY TESTS NOTICE:
// These are legacy MySQL-parity golden tests from the prior architecture.
// They are gated behind the optional `legacy_golden` feature and each test
// is #[ignore] so default CI remains green while the new pipeline-based
// tests replace them.

//!
//! To run these tests:
//! 1. Ensure MySQL is running and accessible
//! 2. Set TEST_DATABASE_URL environment variable (default: mysql://root@localhost)
//! 3. Place packet fixtures in tests/golden/fixtures/
//! 4. Place baseline dumps from Python WeeWX in tests/golden/baselines/
//! 5. Run: cargo test --test golden_tests
//!
//! To update baselines:
//! UPDATE_BASELINES=1 cargo test --test golden_tests

// Note: The golden module is at workspace level in tests/golden/
// This path reference is relative to the workspace root
#[path = "../../../tests/golden/mod.rs"]
mod golden;

use anyhow::Result;
use golden::*;
use weex_archive::IntervalAggregator;
use weex_db::DbClient;

#[tokio::test]
#[ignore] // Requires MySQL and fixtures
async fn test_simple_packet_processing() -> Result<()> {
    let config = GoldenTestConfig::default();

    // Setup test database
    let test_db = test_db::TestDb::new(&config.test_db_url, "simple_packet").await?;
    test_db.init_schema(test_db::weewx_schema()).await?;

    // Load fixture packets
    let fixture_path = config.fixture_path("simple_packet");
    let packets = fixtures::load_packets(&fixture_path)?;

    println!("Loaded {} packets from fixture", packets.len());

    // Create DB client
    let db_client = DbClient::new(&test_db.url()).await?;

    // Process packets through aggregator
    let mut aggregator = IntervalAggregator::new(300, 16, db_client.clone());

    for packet in packets {
        aggregator.add_packet(packet).await?;
    }

    // Force flush to ensure all data is written
    aggregator.force_flush().await?;

    // Dump the database
    let actual_dump = db_diff::DbDump::from_database(&test_db.url()).await?;

    // Load baseline dump
    let baseline_path = config.baseline_path("simple_packet");
    let expected_dump = if baseline_path.exists() {
        db_diff::DbDump::from_file(&baseline_path)?
    } else {
        println!("WARNING: Baseline not found, creating new baseline");
        actual_dump.to_file(&baseline_path)?;
        return Ok(());
    };

    // Compare dumps
    let differences = actual_dump.diff(&expected_dump);

    let result = GoldenTestResult {
        test_name: "simple_packet".to_string(),
        passed: differences.is_empty(),
        differences: differences.clone(),
        actual_dump: actual_dump.to_sql(),
        expected_dump: expected_dump.to_sql(),
    };

    if !result.passed {
        if config.update_baselines {
            println!("Updating baseline due to UPDATE_BASELINES flag");
            actual_dump.to_file(&baseline_path)?;
        } else {
            println!("Differences found:");
            for diff in &differences {
                println!("  - {}", diff);
            }
            result.assert_passed();
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires MySQL and fixtures
async fn test_multi_interval_aggregation() -> Result<()> {
    let config = GoldenTestConfig::default();

    // Setup test database
    let test_db = test_db::TestDb::new(&config.test_db_url, "multi_interval").await?;
    test_db.init_schema(test_db::weewx_schema()).await?;

    // Load fixture packets
    let fixture_path = config.fixture_path("multi_interval");
    let packets = fixtures::load_packets(&fixture_path)?;

    println!("Loaded {} packets from fixture", packets.len());

    // Create DB client
    let db_client = DbClient::new(&test_db.url()).await?;

    // Process packets
    let mut aggregator = IntervalAggregator::new(300, 16, db_client.clone());

    for packet in packets {
        aggregator.add_packet(packet).await?;
    }

    aggregator.force_flush().await?;

    // Verify multiple archive records were created
    let count = test_db.count_rows("archive").await?;
    assert!(
        count > 1,
        "Expected multiple archive records, got {}",
        count
    );

    // Compare with baseline
    let actual_dump = db_diff::DbDump::from_database(&test_db.url()).await?;
    let baseline_path = config.baseline_path("multi_interval");

    if !baseline_path.exists() {
        println!("WARNING: Baseline not found, creating new baseline");
        actual_dump.to_file(&baseline_path)?;
        return Ok(());
    }

    let expected_dump = db_diff::DbDump::from_file(&baseline_path)?;
    let differences = actual_dump.diff(&expected_dump);

    if !differences.is_empty() && !config.update_baselines {
        println!("Differences found:");
        for diff in &differences {
            println!("  - {}", diff);
        }
        panic!("Golden test failed - differences found");
    } else if config.update_baselines {
        actual_dump.to_file(&baseline_path)?;
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires MySQL and fixtures
async fn test_run_all_golden_tests() -> Result<()> {
    let config = GoldenTestConfig::default();

    // Load all fixtures
    let fixtures = fixtures::load_all_fixtures(&config.fixtures_dir)?;

    if fixtures.is_empty() {
        println!("WARNING: No fixtures found in {:?}", config.fixtures_dir);
        return Ok(());
    }

    println!("Running {} golden tests", fixtures.len());

    let mut passed = 0;
    let mut failed = 0;

    for (name, packets) in fixtures {
        println!("\nTesting: {}", name);

        match run_golden_test(&name, &packets, &config).await {
            Ok(true) => {
                println!("  ✓ PASSED");
                passed += 1;
            }
            Ok(false) => {
                println!("  ✗ FAILED");
                failed += 1;
            }
            Err(e) => {
                println!("  ✗ ERROR: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    assert_eq!(failed, 0, "{} test(s) failed", failed);

    Ok(())
}

async fn run_golden_test(
    name: &str,
    packets: &[weex_core::WeatherPacket],
    config: &GoldenTestConfig,
) -> Result<bool> {
    // Setup test database
    let test_db = test_db::TestDb::new(&config.test_db_url, name).await?;
    test_db.init_schema(test_db::weewx_schema()).await?;

    // Create DB client and aggregator
    let db_client = DbClient::new(&test_db.url()).await?;
    let mut aggregator = IntervalAggregator::new(300, 16, db_client);

    // Process packets
    for packet in packets {
        aggregator.add_packet(packet.clone()).await?;
    }

    aggregator.force_flush().await?;

    // Dump and compare
    let actual_dump = db_diff::DbDump::from_database(&test_db.url()).await?;
    let baseline_path = config.baseline_path(name);

    if !baseline_path.exists() {
        if config.update_baselines {
            actual_dump.to_file(&baseline_path)?;
            println!("  Created new baseline");
            return Ok(true);
        } else {
            println!("  Baseline not found: {:?}", baseline_path);
            return Ok(false);
        }
    }

    let expected_dump = db_diff::DbDump::from_file(&baseline_path)?;
    let differences = actual_dump.diff(&expected_dump);

    if !differences.is_empty() {
        if config.update_baselines {
            actual_dump.to_file(&baseline_path)?;
            println!("  Updated baseline");
            return Ok(true);
        } else {
            for diff in &differences {
                println!("    {}", diff);
            }
            return Ok(false);
        }
    }

    Ok(true)
}
