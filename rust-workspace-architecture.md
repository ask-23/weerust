# Rust Workspace Architecture for WeeWX Replacement

## Executive Summary

This document outlines the architectural design for a Rust-based replacement of the WeeWX weather station software. The design prioritizes:

- **Performance**: Tokio async runtime for concurrent I/O operations
- **Type Safety**: Strong typing for units, measurements, and time-series data
- **Database Compatibility**: Direct MySQL integration using existing schema (NO migrations)
- **Modularity**: Clear separation of concerns across crates
- **Extensibility**: Plugin architecture for weather station drivers

## System Context

### Current WeeWX Architecture
- Python 3.7+ monolithic application
- SQLite/MySQL dual database support
- Archive interval-based data aggregation
- Daily summary pre-computation for performance
- Driver abstraction for 50+ weather station types

### Migration Strategy
- Phase 1: Core data structures and database integration (this design)
- Phase 2: Driver abstraction with stubbed implementations
- Phase 3: Archive aggregation engine
- Phase 4: Full daemon with scheduler
- Phase 5: Driver implementations and testing

---

## Workspace Structure

```
weex/
├── Cargo.toml                 # Workspace definition
├── crates/
│   ├── weex-core/            # Data types, units, rollup logic
│   ├── weex-db/              # sqlx MySQL integration
│   ├── weex-ingest/          # Driver adapter trait
│   ├── weex-archive/         # Interval aggregation engine
│   └── weex-daemon/          # Binary with scheduler/writer
├── docs/                      # Architecture documentation
├── tests/                     # Integration tests
└── examples/                  # Usage examples
```

---

## Crate Dependency Graph

```
weex-daemon (bin)
    ├── weex-archive
    │   ├── weex-db
    │   └── weex-core
    ├── weex-ingest
    │   └── weex-core
    └── weex-db
        └── weex-core
```

**Dependency Flow**: daemon → (archive, ingest) → db → core

---

## Crate 1: weex-core

### Purpose
Foundation crate providing data types, unit conversions, and aggregation logic.

### Responsibilities
1. **Type System**: Weather observation types with units
2. **Unit Conversion**: Bi-directional conversions (metric ↔ imperial)
3. **Aggregation Logic**: Rollup calculations (avg, min, max, sum, vector avg)
4. **Time Handling**: Interval calculations and timestamp utilities

### Key Types

```rust
// Core observation record
pub struct Observation {
    pub timestamp: i64,           // Unix timestamp
    pub interval: u32,            // Archive interval (minutes)
    pub temperature: Option<Temperature>,
    pub humidity: Option<Humidity>,
    pub pressure: Option<Pressure>,
    pub wind_speed: Option<Speed>,
    pub wind_dir: Option<Direction>,
    pub rain: Option<Precipitation>,
    // ... additional observation types
}

// Type-safe temperature with unit system
pub struct Temperature {
    value: f32,
    unit: TemperatureUnit,
}

pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

// Aggregation result container
pub struct Aggregate {
    pub avg: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub sum: Option<f32>,
    pub vector_avg: Option<f32>,  // For wind direction
}
```

### Dependencies

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
```

### Architecture Decisions

**ADR-001: Type-Safe Units**
- **Decision**: Newtype pattern for all measurement types
- **Rationale**: Prevents unit confusion errors (e.g., passing Fahrenheit as Celsius)
- **Trade-off**: Increased verbosity vs. compile-time safety

**ADR-002: Option-Based Observations**
- **Decision**: All observation fields are `Option<T>`
- **Rationale**: Weather stations frequently have missing/invalid sensor data
- **Trade-off**: Memory overhead vs. data integrity

---

## Crate 2: weex-db

### Purpose
Database abstraction using sqlx for MySQL with existing schema compatibility.

### Responsibilities
1. **Connection Management**: Connection pooling with sqlx
2. **Schema Queries**: Read existing table structure (NO migrations)
3. **Observation Storage**: Insert/update archive records
4. **Time-Series Queries**: Range queries with interval filtering
5. **Daily Summary**: Read/write pre-computed daily aggregates

### Key Components

```rust
use sqlx::{MySql, Pool};

pub struct ArchiveDb {
    pool: Pool<MySql>,
    table_name: String,
}

impl ArchiveDb {
    // Connect using existing schema
    pub async fn connect(database_url: &str, table_name: &str) -> Result<Self>;

    // Insert observation (existing schema columns)
    pub async fn insert(&self, obs: &Observation) -> Result<()>;

    // Query by time range
    pub async fn query_range(
        &self,
        start: i64,
        end: i64
    ) -> Result<Vec<Observation>>;

    // Daily summary operations
    pub async fn insert_daily_summary(&self, summary: &DailySummary) -> Result<()>;
    pub async fn query_daily_summary(&self, date: NaiveDate) -> Result<DailySummary>;
}
```

### Database Schema Mapping

**Existing WeeWX Archive Table** (DO NOT MODIFY):
```sql
CREATE TABLE archive (
    dateTime INTEGER NOT NULL UNIQUE PRIMARY KEY,
    interval INTEGER NOT NULL,
    outTemp REAL,
    outHumidity REAL,
    barometer REAL,
    windSpeed REAL,
    windDir REAL,
    rain REAL,
    -- ... 40+ additional columns
)
```

**Rust Mapping Strategy**:
- Use `sqlx::query!()` macro for compile-time SQL validation
- Map columns directly to `Observation` struct fields
- Handle NULL values with `Option<T>`
- NO schema migrations - read table structure at runtime

### Dependencies

```toml
[dependencies]
weex-core = { path = "../weex-core" }
sqlx = { version = "0.7", features = ["mysql", "runtime-tokio", "macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
```

### Architecture Decisions

**ADR-003: No Database Migrations**
- **Decision**: Use existing WeeWX schema without modification
- **Rationale**: Enable incremental replacement; coexist with Python WeeWX
- **Trade-off**: Cannot optimize schema vs. migration complexity

**ADR-004: Connection Pooling**
- **Decision**: sqlx connection pooling with configurable pool size
- **Rationale**: Handle concurrent driver writes and query requests
- **Trade-off**: Memory usage vs. connection overhead

---

## Crate 3: weex-ingest

### Purpose
Driver abstraction trait with stubbed implementations for future driver development.

### Responsibilities
1. **Driver Trait**: Common interface for all weather station drivers
2. **Observation Flow**: Standardized data acquisition and validation
3. **Error Handling**: Driver-specific error types
4. **Mock Drivers**: Stub implementations for testing

### Key Trait

```rust
use async_trait::async_trait;
use weex_core::Observation;

#[async_trait]
pub trait WeatherDriver: Send + Sync {
    /// Driver name (e.g., "vantage", "acurite")
    fn name(&self) -> &str;

    /// Initialize hardware connection
    async fn connect(&mut self) -> Result<(), DriverError>;

    /// Read current observation from station
    async fn read_observation(&mut self) -> Result<Observation, DriverError>;

    /// Graceful shutdown
    async fn disconnect(&mut self) -> Result<(), DriverError>;

    /// Optional: Get driver capabilities
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities::default()
    }
}

pub struct DriverCapabilities {
    pub supports_realtime: bool,
    pub supports_archive: bool,
    pub min_interval_seconds: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Read timeout")]
    ReadTimeout,

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Hardware error: {0}")]
    HardwareError(String),
}
```

### Stub Implementations

```rust
// Mock driver for testing
pub struct SimulatorDriver {
    name: String,
}

#[async_trait]
impl WeatherDriver for SimulatorDriver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn connect(&mut self) -> Result<(), DriverError> {
        // Stub: always succeeds
        Ok(())
    }

    async fn read_observation(&mut self) -> Result<Observation, DriverError> {
        // Stub: return synthetic data
        Ok(Observation::synthetic())
    }

    async fn disconnect(&mut self) -> Result<(), DriverError> {
        Ok(())
    }
}

// Vantage driver (stubbed for now)
pub struct VantageDriver {
    port: String,
}

#[async_trait]
impl WeatherDriver for VantageDriver {
    // Stub implementations...
}
```

### Dependencies

```toml
[dependencies]
weex-core = { path = "../weex-core" }
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
thiserror = "1"
```

### Architecture Decisions

**ADR-005: Async Driver Trait**
- **Decision**: Use `async_trait` for async trait methods
- **Rationale**: Enable async I/O in driver implementations
- **Trade-off**: Runtime overhead vs. modern async ecosystem

**ADR-006: Stub-First Development**
- **Decision**: Provide stub implementations for all planned drivers
- **Rationale**: Enable testing and development without hardware
- **Trade-off**: Initial development time vs. testability

---

## Crate 4: weex-archive

### Purpose
Interval-based aggregation engine for computing statistics from raw observations.

### Responsibilities
1. **Interval Aggregation**: Compute avg/min/max/sum over time windows
2. **Vector Averaging**: Wind direction circular averaging
3. **Daily Rollup**: Generate daily summaries from archive records
4. **Accumulation**: Rainfall accumulation and delta calculations

### Key Components

```rust
use weex_core::{Observation, Aggregate};
use weex_db::ArchiveDb;

pub struct ArchiveEngine {
    db: ArchiveDb,
    interval_minutes: u32,
}

impl ArchiveEngine {
    pub fn new(db: ArchiveDb, interval_minutes: u32) -> Self;

    /// Aggregate observations within interval into single archive record
    pub async fn aggregate_interval(
        &self,
        observations: Vec<Observation>
    ) -> Result<Observation>;

    /// Compute daily summary from archive records
    pub async fn compute_daily_summary(
        &self,
        date: NaiveDate
    ) -> Result<DailySummary>;

    /// Rainfall accumulation calculation
    pub fn compute_rainfall_delta(
        &self,
        current: f32,
        previous: f32
    ) -> f32;

    /// Vector average for wind direction
    pub fn vector_average(&self, directions: &[f32]) -> Option<f32>;
}

pub struct DailySummary {
    pub date: NaiveDate,
    pub temp_avg: Option<f32>,
    pub temp_min: Option<f32>,
    pub temp_max: Option<f32>,
    pub rainfall_total: Option<f32>,
    pub wind_max_gust: Option<f32>,
    // ... additional daily aggregates
}
```

### Aggregation Strategy

**Interval Processing**:
1. Collect observations within archive interval (e.g., 5 minutes)
2. Compute statistics per observation type:
   - Temperature: avg, min, max
   - Humidity: avg
   - Pressure: avg
   - Wind speed: avg, max (gust)
   - Wind direction: vector average
   - Rain: delta/accumulation
3. Store aggregated record in archive table

**Daily Summary**:
1. Query all archive records for calendar date
2. Compute daily statistics across intervals
3. Store in daily summary table for fast queries

### Dependencies

```toml
[dependencies]
weex-core = { path = "../weex-core" }
weex-db = { path = "../weex-db" }
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
```

### Architecture Decisions

**ADR-007: Pre-Computed Daily Summaries**
- **Decision**: Maintain separate daily summary table
- **Rationale**: Dramatically improves query performance for historical data
- **Trade-off**: Storage space vs. query speed (WeeWX design pattern)

**ADR-008: Vector Averaging for Wind**
- **Decision**: Circular mean calculation for wind direction
- **Rationale**: Mathematically correct averaging of angular measurements
- **Trade-off**: Computational complexity vs. accuracy

---

## Crate 5: weex-daemon

### Purpose
Main binary with scheduler, writer threads, and driver coordination.

### Responsibilities
1. **Configuration Loading**: Parse TOML config file
2. **Driver Management**: Initialize and poll weather station driver
3. **Scheduler**: Interval-based archiving with tokio timers
4. **Writer Thread**: Async database writes without blocking reads
5. **Signal Handling**: Graceful shutdown on SIGTERM/SIGINT

### Architecture

```rust
use tokio::{sync::mpsc, time};
use weex_ingest::WeatherDriver;
use weex_archive::ArchiveEngine;

pub struct WeeXDaemon {
    driver: Box<dyn WeatherDriver>,
    engine: ArchiveEngine,
    config: DaemonConfig,
    observation_tx: mpsc::Sender<Observation>,
}

pub struct DaemonConfig {
    pub archive_interval_minutes: u32,
    pub database_url: String,
    pub driver_name: String,
    pub driver_config: DriverConfig,
}

impl WeeXDaemon {
    pub async fn new(config: DaemonConfig) -> Result<Self>;

    /// Main daemon loop
    pub async fn run(&mut self) -> Result<()> {
        // Spawn writer task
        let writer_handle = tokio::spawn(self.writer_task());

        // Spawn scheduler task
        let scheduler_handle = tokio::spawn(self.scheduler_task());

        // Wait for shutdown signal
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received");
            }
        }

        // Graceful shutdown
        self.shutdown().await
    }

    /// Observation writer task
    async fn writer_task(&self) -> Result<()> {
        // Consume observations from channel and write to database
    }

    /// Archive scheduler task
    async fn scheduler_task(&self) -> Result<()> {
        // Periodic interval-based archiving
    }
}
```

### Task Flow

```
┌─────────────────┐
│  Driver Poll    │ (every N seconds)
│  read_observation() │
└────────┬────────┘
         │ Observation
         ▼
┌─────────────────┐
│  Channel (mpsc) │ (non-blocking buffer)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Writer Task    │ (async DB writes)
│  insert()       │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  MySQL Database │
└─────────────────┘

┌─────────────────┐
│ Scheduler Task  │ (interval timer)
│ aggregate_interval() │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Archive Engine  │
│ compute_daily_summary() │
└─────────────────┘
```

### Dependencies

```toml
[dependencies]
weex-core = { path = "../crates/weex-core" }
weex-db = { path = "../crates/weex-db" }
weex-ingest = { path = "../crates/weex-ingest" }
weex-archive = { path = "../crates/weex-archive" }

tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Architecture Decisions

**ADR-009: Tokio Runtime**
- **Decision**: Use tokio async runtime for all I/O
- **Rationale**: Maximize concurrency for driver I/O, DB writes, scheduling
- **Trade-off**: Runtime complexity vs. performance

**ADR-010: Channel-Based Writer**
- **Decision**: mpsc channel between driver poll and DB writer
- **Rationale**: Decouple driver reads from DB writes; prevent blocking
- **Trade-off**: Memory buffering vs. backpressure handling

**ADR-011: Configuration as Code**
- **Decision**: TOML-based configuration with strong typing
- **Rationale**: Type-safe config, easier validation than INI files
- **Trade-off**: WeeWX compatibility vs. Rust ecosystem standards

---

## Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/weex-core",
    "crates/weex-db",
    "crates/weex-ingest",
    "crates/weex-archive",
    "crates/weex-daemon",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "GPL-3.0"
authors = ["WeeWX Contributors"]

[workspace.dependencies]
# Shared dependencies across crates
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.dev]
opt-level = 0
```

---

## Build and Development

### Build Commands

```bash
# Build entire workspace
cargo build --workspace

# Build release binary
cargo build --release -p weex-daemon

# Run tests
cargo test --workspace

# Run specific crate tests
cargo test -p weex-core

# Run daemon with example config
cargo run -p weex-daemon -- --config config.toml
```

### Development Workflow

1. **Phase 1**: Implement `weex-core` with unit tests
2. **Phase 2**: Implement `weex-db` with integration tests against test MySQL
3. **Phase 3**: Implement `weex-ingest` trait and simulator driver
4. **Phase 4**: Implement `weex-archive` with aggregation logic
5. **Phase 5**: Implement `weex-daemon` with end-to-end testing

---

## Testing Strategy

### Unit Tests
- **weex-core**: Unit conversions, aggregation math, type safety
- **weex-db**: Mock database interactions with sqlx test macros
- **weex-ingest**: Driver trait implementations with synthetic data
- **weex-archive**: Aggregation correctness with known datasets

### Integration Tests
- **weex-db**: Real MySQL connection with Docker test container
- **weex-archive**: End-to-end aggregation with test database
- **weex-daemon**: Full daemon lifecycle with simulator driver

### Test Database Setup

```bash
# Docker MySQL for testing
docker run -d \
  --name weex-test-mysql \
  -e MYSQL_ROOT_PASSWORD=weewx \
  -e MYSQL_DATABASE=weewx \
  -p 3306:3306 \
  mysql:8

# Use existing WeeWX schema dump
mysql -h localhost -u root -pweewx weewx < schema.sql
```

---

## Performance Considerations

### Database Connection Pooling
- **Pool Size**: 5-10 connections (configurable)
- **Idle Timeout**: 60 seconds
- **Max Lifetime**: 30 minutes

### Memory Management
- **Observation Buffer**: mpsc channel bounded to 1000 observations
- **Backpressure**: Driver polls block if buffer full
- **Archive Batch Size**: Process 100 records per daily summary computation

### Async Optimization
- **Driver Polling**: Async sleep between reads (non-blocking)
- **Database Writes**: Batched inserts where possible
- **Scheduler**: Tokio interval timers (low overhead)

---

## Security Considerations

### Database Credentials
- **Environment Variables**: `DATABASE_URL` not in config file
- **Connection String**: TLS/SSL support via sqlx
- **Permissions**: Read/write only to archive tables

### Driver Security
- **USB Permissions**: Require explicit device permissions
- **Network Drivers**: Firewall rules for weather station IPs
- **Serial Port Access**: Group-based permissions (dialout/uucp)

---

## Future Extensions

### Phase 2 Enhancements
1. **Driver Implementations**:
   - Vantage (Davis)
   - AcuRite
   - Fine Offset

2. **RESTful API**:
   - axum web framework
   - JSON observation endpoints
   - Historical query API

3. **Metrics/Observability**:
   - Prometheus metrics
   - Structured logging (tracing)
   - Health check endpoints

4. **Configuration Hot-Reload**:
   - Watch config file for changes
   - Graceful driver restart

---

## Migration Path from Python WeeWX

### Coexistence Strategy
1. **Read-Only Phase**: Rust daemon reads from Python-created database
2. **Write Coexistence**: Both systems write to same database
3. **Read Migration**: Rust daemon becomes primary, Python reads for comparison
4. **Full Migration**: Python WeeWX deprecated

### Compatibility Requirements
- ✅ Use existing database schema (no modifications)
- ✅ Support both SQLite and MySQL (Phase 1: MySQL only)
- ✅ Maintain archive interval semantics
- ✅ Preserve daily summary computation logic

---

## Conclusion

This architecture provides a solid foundation for a high-performance, type-safe WeeWX replacement while maintaining compatibility with existing deployments. The modular design enables incremental development and testing, with clear separation of concerns across crates.

**Next Steps**:
1. Create workspace directory structure
2. Implement `weex-core` data types and unit tests
3. Set up CI/CD pipeline with GitHub Actions
4. Implement `weex-db` with test MySQL container

---

## Appendix: Architecture Decision Records

| ADR | Decision | Rationale |
|-----|----------|-----------|
| ADR-001 | Type-safe units with newtype pattern | Compile-time unit safety |
| ADR-002 | Option-based observations | Handle missing sensor data |
| ADR-003 | No database migrations | Coexist with Python WeeWX |
| ADR-004 | Connection pooling | Concurrent DB access |
| ADR-005 | Async driver trait | Modern async I/O |
| ADR-006 | Stub-first driver development | Testability without hardware |
| ADR-007 | Pre-computed daily summaries | Query performance |
| ADR-008 | Vector averaging for wind | Mathematical correctness |
| ADR-009 | Tokio async runtime | Maximize concurrency |
| ADR-010 | Channel-based writer | Decouple reads from writes |
| ADR-011 | TOML configuration | Type-safe config |

---

**Document Version**: 1.0
**Last Updated**: 2025-10-14
**Author**: System Architect Agent
**Status**: Draft for Review
