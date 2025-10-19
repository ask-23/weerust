# Golden Tests for WeeWX MySQL Integration

This directory contains golden tests that validate the MySQL database integration by comparing actual database dumps against known-good baselines.

## Directory Structure

```
tests/golden/
├── packets/           # Captured JSON weather packets
├── baseline/          # Expected database dumps
│   └── expected_dump.sql
├── src/
│   └── main.rs       # Test runner implementation
├── Cargo.toml
└── README.md
```

## How It Works

1. **Load Packets**: Reads all JSON files from `packets/` directory
2. **Create Test Database**: Sets up a temporary MySQL database with WeeWX schema
3. **Write Data**: Inserts weather packets into the test database
4. **Export Dump**: Creates a mysqldump of the test database
5. **Compare**: Diffs the actual dump against the baseline in `baseline/expected_dump.sql`
6. **Cleanup**: Drops the test database

## Running Tests

### Prerequisites

```bash
# MySQL server must be running
# Set database connection (optional, defaults to localhost)
export DATABASE_URL="mysql://user:pass@localhost:3306"
```

### Execute Tests

```bash
cd tests/golden
cargo run
```

### Expected Output

```
🧪 Golden Test Runner

1️⃣  Setting up test database...
✅ Test database 'weewx_golden_test' created successfully

2️⃣  Loading JSON packets...
📦 Loaded packet: ../packets/packet_001.json
📦 Loaded packet: ../packets/packet_002.json
✅ Loaded 2 packets

3️⃣  Writing packets to database...
✅ Wrote 2 packets to database

4️⃣  Exporting database dump...
✅ Exported database to ../baseline/actual_dump.sql

5️⃣  Comparing with baseline...
✅ Database dumps match baseline

6️⃣  Cleaning up...
🧹 Cleaned up test database

✨ All golden tests passed!
```

## Adding New Test Cases

### 1. Capture Packets

Add JSON files to `packets/` directory:

```json
{
  "timestamp": 1704067200,
  "temperature": 72.5,
  "humidity": 45.0,
  "pressure": 1013.25,
  "wind_speed": 5.2,
  "wind_direction": 180.0,
  "rain": 0.0
}
```

### 2. Run Tests

```bash
cargo run
```

### 3. Update Baseline (if changes are expected)

```bash
cp ../baseline/actual_dump.sql ../baseline/expected_dump.sql
```

## Database Schema

The test uses the WeeWX archive table schema:

```sql
CREATE TABLE archive (
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
);
```

## Troubleshooting

### Test Failures

If tests fail with database diff:

1. Check `baseline/actual_dump.diff` for differences
2. Verify packet JSON format matches schema
3. Ensure MySQL server is running and accessible
4. Check database permissions

### Updating Baseline

When intentional changes are made to the MySQL integration:

```bash
# Review the changes first
diff baseline/expected_dump.sql baseline/actual_dump.sql

# Update baseline if changes are correct
cp baseline/actual_dump.sql baseline/expected_dump.sql
```

## Integration with CI/CD

Add to your CI pipeline:

```yaml
- name: Run Golden Tests
  run: |
    cd tests/golden
    cargo run
```

## Notes

- Tests automatically clean up the test database
- Dumps are normalized to ignore timestamps and auto-increment values
- All packets must have valid timestamps (Unix epoch)
- JSON packets are sorted by timestamp before insertion
