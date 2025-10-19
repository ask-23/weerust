# WeeWX Rust Port

A Rust implementation of WeeWX weather station software, maintaining strict parity with the Python MySQL schema and output.

## Project Goals

1. **Language Port**: Migrate core WeeWX functionality from Python to Rust
2. **Containerization**: Package as containerized service
3. **Schema Parity**: Maintain 100% compatibility with existing MySQL schema
4. **Test-Driven**: Golden test suite validates output against Python WeeWX

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   weex-daemon                       │
│  (scheduler + archive writer)                       │
└──────────────┬────────────────┬─────────────────────┘
               │                │
    ┌──────────▼─────┐   ┌─────▼────────┐
    │ weex-ingest    │   │ weex-archive │
    │ (drivers)      │   │ (aggregator) │
    └──────┬─────────┘   └─────┬────────┘
           │                   │
           │      ┌────────────▼──────┐
           │      │    weex-db        │
           │      │  (MySQL access)   │
           │      └────────┬──────────┘
           │               │
       ┌───▼───────────────▼───┐
       │     weex-core         │
       │ (types, units, math)  │
       └───────────────────────┘
```

## Crates

### weex-core
Core data types, unit conversions, and aggregation algorithms.

**Key Components:**
- `types.rs`: Weather packet and archive record structures
- `units.rs`: Unit system conversions (US ↔ Metric)
- `rollups.rs`: Aggregation accumulators (min, max, avg, sum)

### weex-db
Database access layer using sqlx for MySQL.

**Key Features:**
- NO migrations - uses existing Python WeeWX schema
- Connection pooling and retry logic
- Archive and metadata table operations

### weex-ingest
Weather station driver adapters.

**Status:** Currently stubbed with simulator driver
**Future:** Vantage, Davis, Acurite, etc. protocol implementations

### weex-archive
Interval aggregation engine.

**Functionality:**
- Buffers incoming packets
- Detects interval boundaries
- Aggregates observations
- Writes archive records

### weex-daemon
Main binary executable.

**Components:**
- Scheduler: coordinates packet collection
- Configuration: environment-based settings
- Signal handling: graceful shutdown

## Golden Test Suite

The golden test harness validates Rust output against Python WeeWX baseline.

### Test Flow

```
1. Load packet JSON (captured from Python WeeWX)
   ↓
2. Process through Rust implementation
   ↓
3. Write to test MySQL database
   ↓
4. Dump database state
   ↓
5. Compare with baseline dump from Python
   ↓
6. Report differences (field-by-field)
```

### Running Golden Tests

```bash
# Prerequisites
# - MySQL server running
# - TEST_DATABASE_URL set (default: mysql://root@localhost)

# Run tests
cargo test --test golden_tests -- --ignored

# Update baselines (after verifying correctness)
UPDATE_BASELINES=1 cargo test --test golden_tests -- --ignored
```

### Creating Test Fixtures

1. Capture packets from Python WeeWX (see `tests/golden/fixtures/README.md`)
2. Place JSON in `tests/golden/fixtures/`
3. Run Python WeeWX to generate baseline
4. Export database: `mysqldump weewx > tests/golden/baselines/test_name.sql`
5. Run golden tests

## Development Setup

### Prerequisites

- Rust 1.70+ (2021 edition)
- MySQL 5.7+ or MariaDB 10.3+
- Docker (optional, for containerization)

### Build

```bash
# Build all crates
cargo build

# Build with optimizations
cargo build --release

# Build specific crate
cargo build -p weex-daemon
```

### Testing

```bash
# Unit tests
cargo test

# Integration tests (requires MySQL)
cargo test --test golden_tests -- --ignored

# Specific crate tests
cargo test -p weex-core
```

### Running the Daemon

```bash
# Set required environment variables
export DATABASE_URL="mysql://weewx:password@localhost/weewx"
export ARCHIVE_INTERVAL=300  # seconds
export POLL_INTERVAL=10      # seconds
export UNIT_SYSTEM=16        # 1=US, 16=Metric, 17=MetricWX
export STATION_DRIVER=simulator

# Run daemon
cargo run --bin weexd

# Or with logging
RUST_LOG=info cargo run --bin weexd
```

## Configuration

All configuration via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | (required) | MySQL connection string |
| `ARCHIVE_INTERVAL` | 300 | Archive interval in seconds |
| `POLL_INTERVAL` | 10 | Driver poll interval |
| `UNIT_SYSTEM` | 16 | Unit system (1=US, 16=Metric) |
| `STATION_DRIVER` | simulator | Driver type |
| `RUST_LOG` | info | Log level |

## Database Schema

Uses existing Python WeeWX schema - **NO migrations**.

### Archive Table

```sql
CREATE TABLE archive (
    dateTime INT NOT NULL PRIMARY KEY,
    usUnits INT NOT NULL,
    `interval` INT NOT NULL,
    outTemp REAL,
    outHumidity REAL,
    barometer REAL,
    windSpeed REAL,
    windDir REAL,
    rain REAL,
    -- ... (all standard WeeWX fields)
);
```

Schema must be created by Python WeeWX or manually before running Rust version.

## Containerization

**Status:** Planned

**Target:**
- Multi-stage Dockerfile with minimal runtime image
- Docker Compose setup with MySQL
- Volume mounts for configuration
- Health checks and auto-restart

## Constraints

### Strict Parity Requirements

1. **No New Features**: Only port existing functionality
2. **Schema Compatibility**: 100% compatible with Python WeeWX database
3. **Calculation Parity**: Aggregations must match Python exactly
4. **Golden Tests**: All tests must pass before merging

### What We Don't Port (Yet)

- Reports and templating
- FTP/RSYNC upload
- RESTful services
- Extensions and plugins
- Web UI

Focus is core data collection and archiving only.

## Performance Targets

- Memory: < 50MB RSS
- CPU: < 5% on average
- Startup: < 1 second
- Aggregation latency: < 100ms

## Contributing

1. Follow existing code style (rustfmt)
2. Add unit tests for new functionality
3. Update golden tests if changing aggregation logic
4. Verify all tests pass: `cargo test`
5. Check clippy: `cargo clippy`

## License

GPL-3.0 (matching Python WeeWX)

## References

- [Python WeeWX](http://www.weewx.com/)
- [WeeWX Database Schema](http://www.weewx.com/docs/customizing.htm#archive_database)
- [sqlx Documentation](https://docs.rs/sqlx/)
- [Tokio Runtime](https://tokio.rs/)

## Project Status

**Phase:** Initial Implementation
- ✅ Workspace structure
- ✅ Core data types and units
- ✅ Database access layer
- ✅ Aggregation engine
- ✅ Golden test harness
- ✅ Simulator driver
- ⏳ Real hardware drivers
- ⏳ Containerization
- ⏳ Production testing

## Quick Start

```bash
# 1. Clone repository
git clone <repo> && cd weewx

# 2. Setup test database
mysql -e "CREATE DATABASE weewx_test"
mysql weewx_test < schema/weewx.sql

# 3. Run unit tests
cargo test

# 4. Run golden tests (with fixtures)
TEST_DATABASE_URL=mysql://root@localhost cargo test --test golden_tests -- --ignored

# 5. Run daemon (simulator mode)
DATABASE_URL=mysql://root@localhost/weewx cargo run --bin weexd
```
