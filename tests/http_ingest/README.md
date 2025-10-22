# WeeRust HTTP POST Ingestion Test Suite

Comprehensive testing suite for validating HTTP POST endpoint ingestion of weather data from Ecowitt GW1100 and Weather Underground compatible devices.

## Overview

This test suite validates:
- **Data Format Support**: Ecowitt GW1100 and Weather Underground formats
- **Error Handling**: Malformed data, invalid types, edge cases
- **Performance**: Concurrent requests, stress testing
- **Data Persistence**: MariaDB insertion and integrity
- **Logging**: Verification of logging completeness
- **Integration**: End-to-end data flow validation

## Test Files

### Rust Integration Tests
- **`http_post_ingest.rs`**: Comprehensive Rust integration tests
  - 13 test functions covering all scenarios
  - Uses axum test framework with tower ServiceExt
  - Tests run against in-memory test instance

### Bash Test Scripts

#### Format Validation
- **`test_ecowitt_format.sh`**: Ecowitt GW1100 format tests (15 scenarios)
  - Complete sensor coverage (temp, humidity, wind, rain, solar, etc.)
  - Battery levels, PM2.5, lightning detection
  - Extreme values and special characters

- **`test_wunderground_format.sh`**: Weather Underground format tests (12 scenarios)
  - WU authentication and parameters
  - Imperial and metric units
  - Indoor measurements, soil sensors, AQI

#### Error Handling
- **`test_error_handling.sh`**: Error scenario tests (19 scenarios)
  - Malformed data and invalid types
  - Security (SQL injection, XSS)
  - Unicode, null bytes, binary data
  - Wrong content types and HTTP methods

#### Performance
- **`test_stress.sh`**: Stress and load testing
  - Concurrent request testing (configurable)
  - Response time measurement
  - Sustained load testing (30 seconds)
  - Performance statistics

#### Validation
- **`validate_mariadb.sh`**: Database integrity checks
  - Schema validation
  - Data type verification
  - Timestamp ordering and gaps
  - Statistical analysis
  - Index and optimization checks

- **`validate_logging.sh`**: Log verification
  - Startup and binding logs
  - Request and insertion logging
  - Error detection
  - Performance metrics
  - Log statistics

#### Test Runner
- **`run_all_tests.sh`**: Master test orchestrator
  - Runs all test suites sequentially
  - Generates comprehensive Markdown report
  - Tracks overall pass/fail status
  - Pre-flight server health check

## Quick Start

### Prerequisites

```bash
# Ensure server is running
docker-compose up -d

# Or if running locally
cargo run --release
```

### Run All Tests

```bash
cd tests/http_ingest
chmod +x *.sh
./run_all_tests.sh http://localhost:8080
```

### Run Individual Tests

```bash
# Ecowitt format validation
./test_ecowitt_format.sh http://localhost:8080

# Weather Underground format
./test_wunderground_format.sh http://localhost:8080

# Error handling
./test_error_handling.sh http://localhost:8080

# Stress test (100 requests, 10 concurrent)
./test_stress.sh http://localhost:8080 100 10

# Database validation
./validate_mariadb.sh

# Logging validation
./validate_logging.sh container
```

### Run Rust Tests

```bash
cd ../../crates/weewx-cli
cargo test http_post_ingest
```

## Test Coverage

### Data Format Tests (27 scenarios)
- ✅ Ecowitt GW1100 format (15 tests)
- ✅ Weather Underground format (12 tests)

### Error Handling Tests (19 scenarios)
- ✅ Malformed data (3 tests)
- ✅ Invalid data types (5 tests)
- ✅ Security tests (2 tests)
- ✅ Edge cases (9 tests)

### Performance Tests (4 scenarios)
- ✅ Concurrent requests
- ✅ Response time analysis
- ✅ Sustained load
- ✅ Throughput measurement

### System Validation Tests (25+ checks)
- ✅ Database integrity (15 checks)
- ✅ Logging completeness (10 checks)

**Total Test Scenarios: 75+**

## Expected Behavior

### Successful Tests
All tests should pass with 200 OK status codes for valid data.

### Error Handling
- Invalid data should be handled gracefully (200 OK with partial data or 400 Bad Request)
- Security attempts (SQL injection, XSS) should be safely rejected
- Malformed requests should not crash the server

### Performance Targets
- Response time: < 100ms for single request
- Concurrent handling: 100 requests at 10 parallelism with 100% success
- Sustained load: > 5 requests/second for 30 seconds

### Database Validation
- All POSTed data should appear in MariaDB `archive` table
- Timestamps should be ordered correctly
- No data corruption or type mismatches
- Proper indexing and optimization

### Logging Requirements
- All POST requests logged
- Data insertion logged (if INSERT_LOGGING=true)
- No critical errors in logs
- Performance metrics available

## Test Report

After running `run_all_tests.sh`, a comprehensive Markdown report is generated:

```
tests/http_ingest/test_report_YYYYMMDD_HHMMSS.md
```

The report includes:
- Executive summary
- Test results matrix
- Detailed coverage breakdown
- Performance metrics
- Recommendations
- Test commands reference

## Environment Variables

Tests respect the following environment variables:

```bash
# Server configuration
LISTEN_PORT=8080                    # HTTP server port

# Database configuration
DB_HOST=localhost                   # MariaDB host
DB_PORT=3306                        # MariaDB port
DB_NAME=weewx                       # Database name
DB_USER=weewx                       # Database user
DB_PASS=weewxpass                   # Database password

# Logging configuration
INSERT_LOGGING=true                 # Enable insertion logging
RUST_LOG=info                       # Log level

# Station configuration
STATION_FORMAT=ecowitt              # ecowitt | wunderground
```

## Troubleshooting

### Server Not Accessible
```bash
# Check server status
curl http://localhost:8080/healthz

# Check Docker containers
docker ps
docker logs weerust-rust
```

### Database Connection Failures
```bash
# Test database connection
mysql -h localhost -u weewx -pweewxpass -D weewx -e "SELECT 1"

# Check MariaDB container
docker logs weerust-mariadb
```

### Permission Errors
```bash
# Make all scripts executable
chmod +x *.sh
```

### Missing Dependencies
```bash
# Install required tools
brew install coreutils gnu-parallel  # macOS
apt-get install parallel mysql-client  # Ubuntu/Debian
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: HTTP Ingest Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Start services
        run: docker-compose up -d

      - name: Wait for services
        run: sleep 10

      - name: Run tests
        run: |
          cd tests/http_ingest
          chmod +x *.sh
          ./run_all_tests.sh http://localhost:8080

      - name: Upload test report
        uses: actions/upload-artifact@v3
        with:
          name: test-report
          path: tests/http_ingest/test_report_*.md
```

## Contributing

When adding new tests:

1. Add test scenarios to appropriate script
2. Update test counters
3. Document expected behavior
4. Update this README
5. Ensure all tests pass before committing

## Test Data Examples

### Valid Ecowitt POST
```
stationtype=GW1100&
dateutc=now&
tempf=78.6&
humidity=52&
winddir=180&
windspeedmph=3.2&
baromabsin=29.92&
solarradiation=120.5&
uv=2
```

### Valid Weather Underground POST
```
ID=STATION123&
PASSWORD=mypass&
dateutc=now&
tempf=72.5&
baromin=29.92&
humidity=55&
windspeedmph=5.0&
windgustmph=7.0&
winddir=180
```

## Support

For issues or questions:
- Check test reports for specific failure details
- Review server logs: `docker logs weerust-rust`
- Review database: `./validate_mariadb.sh`
- Open an issue with test report attached

---

**Last Updated**: 2025-10-21
**Test Suite Version**: 1.0.0
**Maintained By**: WeeRust Testing Team
