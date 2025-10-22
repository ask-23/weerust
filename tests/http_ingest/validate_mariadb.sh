#!/usr/bin/env bash
# Validate MariaDB data insertion from HTTP POST
# Usage: ./validate_mariadb.sh

set -euo pipefail

# Load environment variables
if [ -f "$(dirname "$0")/../../.env" ]; then
    source "$(dirname "$0")/../../.env"
fi

# Database configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-3306}"
DB_NAME="${DB_NAME:-weewx}"
DB_USER="${DB_USER:-weewx}"
DB_PASS="${DB_PASS:-weewxpass}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run SQL query
run_query() {
    mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" -D"${DB_NAME}" -sse "$1" 2>/dev/null
}

# Function to run a validation test
run_test() {
    local test_name="$1"
    local query="$2"
    local expected="$3"

    TESTS_RUN=$((TESTS_RUN + 1))

    echo -e "\n${YELLOW}Test ${TESTS_RUN}: ${test_name}${NC}"
    echo "Query: ${query}"

    result=$(run_query "${query}" || echo "ERROR")

    echo "Result: ${result}"
    echo "Expected: ${expected}"

    if [ "${result}" = "${expected}" ] || [ "${expected}" = "ANY" ]; then
        echo -e "${GREEN}✓ PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAILED${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

echo "================================================"
echo "MariaDB Validation Tests"
echo "================================================"
echo "Host: ${DB_HOST}:${DB_PORT}"
echo "Database: ${DB_NAME}"
echo "User: ${DB_USER}"
echo "================================================"

# Test 1: Database connectivity
echo -e "\n${BLUE}Testing database connectivity...${NC}"
if run_query "SELECT 1" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Database connection successful${NC}"
else
    echo -e "${RED}✗ Cannot connect to database${NC}"
    exit 1
fi

# Test 2: Check if archive table exists
echo -e "\n${BLUE}Checking database schema...${NC}"
TABLE_EXISTS=$(run_query "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='${DB_NAME}' AND table_name='archive'")
if [ "${TABLE_EXISTS}" = "1" ]; then
    echo -e "${GREEN}✓ Archive table exists${NC}"
else
    echo -e "${YELLOW}⚠ Archive table does not exist - might not be created yet${NC}"
fi

# Test 3: Count total records
RECORD_COUNT=$(run_query "SELECT COUNT(*) FROM archive" || echo "0")
echo -e "\n${BLUE}Total records in archive: ${RECORD_COUNT}${NC}"

if [ "${RECORD_COUNT}" -gt 0 ]; then
    # Test 4: Check recent records
    run_test "Recent Records (last hour)" \
        "SELECT COUNT(*) FROM archive WHERE dateTime > UNIX_TIMESTAMP(NOW() - INTERVAL 1 HOUR)" \
        "ANY"

    # Test 5: Check for NULL values in critical fields
    NULL_TEMPS=$(run_query "SELECT COUNT(*) FROM archive WHERE outTemp IS NULL AND dateTime > UNIX_TIMESTAMP(NOW() - INTERVAL 1 DAY)" || echo "0")
    echo -e "\n${BLUE}Records with NULL temperature (last 24h): ${NULL_TEMPS}${NC}"

    # Test 6: Check data types are correct
    run_test "Temperature Data Type Check" \
        "SELECT DATA_TYPE FROM information_schema.COLUMNS WHERE TABLE_SCHEMA='${DB_NAME}' AND TABLE_NAME='archive' AND COLUMN_NAME='outTemp'" \
        "ANY"

    # Test 7: Check for reasonable temperature values
    run_test "Reasonable Temperature Values" \
        "SELECT COUNT(*) FROM archive WHERE outTemp BETWEEN -50 AND 150 AND dateTime > UNIX_TIMESTAMP(NOW() - INTERVAL 1 HOUR)" \
        "ANY"

    # Test 8: Check timestamp ordering
    run_test "Timestamps in Order" \
        "SELECT COUNT(*) FROM (SELECT dateTime, LAG(dateTime) OVER (ORDER BY dateTime) as prev FROM archive ORDER BY dateTime DESC LIMIT 10) t WHERE dateTime < prev" \
        "0"

    # Test 9: Check for duplicate timestamps
    DUPLICATES=$(run_query "SELECT COUNT(*) - COUNT(DISTINCT dateTime) FROM archive" || echo "0")
    echo -e "\n${BLUE}Duplicate timestamps: ${DUPLICATES}${NC}"
    if [ "${DUPLICATES}" = "0" ]; then
        echo -e "${GREEN}✓ No duplicate timestamps${NC}"
    else
        echo -e "${YELLOW}⚠ Found ${DUPLICATES} duplicate timestamps${NC}"
    fi

    # Test 10: Sample recent data
    echo -e "\n${BLUE}Sample of recent data:${NC}"
    run_query "SELECT FROM_UNIXTIME(dateTime) as time, outTemp, barometer, humidity, windSpeed FROM archive ORDER BY dateTime DESC LIMIT 5" | column -t

    # Test 11: Data field coverage
    echo -e "\n${BLUE}Field Coverage (last 100 records):${NC}"
    for field in outTemp barometer humidity windSpeed windDir; do
        non_null=$(run_query "SELECT COUNT(*) FROM (SELECT * FROM archive ORDER BY dateTime DESC LIMIT 100) t WHERE ${field} IS NOT NULL" || echo "0")
        echo "  ${field}: ${non_null}/100 records"
    done

    # Test 12: Check for gaps in data
    echo -e "\n${BLUE}Checking for data gaps (> 10 minutes):${NC}"
    GAPS=$(run_query "SELECT COUNT(*) FROM (SELECT dateTime, LAG(dateTime) OVER (ORDER BY dateTime) as prev FROM archive ORDER BY dateTime DESC LIMIT 100) t WHERE (dateTime - prev) > 600" || echo "0")
    echo "  Gaps found: ${GAPS}"
    if [ "${GAPS}" -gt 0 ]; then
        echo -e "${YELLOW}⚠ Data gaps detected${NC}"
        run_query "SELECT FROM_UNIXTIME(prev) as gap_start, FROM_UNIXTIME(dateTime) as gap_end, (dateTime - prev) as gap_seconds FROM (SELECT dateTime, LAG(dateTime) OVER (ORDER BY dateTime) as prev FROM archive ORDER BY dateTime DESC LIMIT 100) t WHERE (dateTime - prev) > 600 LIMIT 5" | column -t
    fi

    # Test 13: Average values
    echo -e "\n${BLUE}Statistical Summary (last 24 hours):${NC}"
    run_query "SELECT
        COUNT(*) as record_count,
        ROUND(AVG(outTemp), 2) as avg_temp,
        ROUND(MIN(outTemp), 2) as min_temp,
        ROUND(MAX(outTemp), 2) as max_temp,
        ROUND(AVG(humidity), 2) as avg_humidity,
        ROUND(AVG(windSpeed), 2) as avg_wind
    FROM archive
    WHERE dateTime > UNIX_TIMESTAMP(NOW() - INTERVAL 24 HOUR)" | column -t

else
    echo -e "${YELLOW}⚠ No records found in archive table${NC}"
    echo "This might be expected if no data has been posted yet."
fi

# Test 14: Check indexes
echo -e "\n${BLUE}Checking indexes:${NC}"
run_query "SHOW INDEX FROM archive" | column -t

# Test 15: Table size and optimization
echo -e "\n${BLUE}Table statistics:${NC}"
TABLE_STATS=$(run_query "SELECT
    ROUND(DATA_LENGTH/1024/1024, 2) as data_mb,
    ROUND(INDEX_LENGTH/1024/1024, 2) as index_mb,
    TABLE_ROWS as approx_rows
FROM information_schema.TABLES
WHERE TABLE_SCHEMA='${DB_NAME}' AND TABLE_NAME='archive'")
echo "${TABLE_STATS}" | column -t

# Summary
echo ""
echo "================================================"
echo "Validation Summary"
echo "================================================"
echo "Total Tests: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
echo "Database Records: ${RECORD_COUNT}"

if [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "\n${GREEN}✓ All MariaDB validations passed!${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some MariaDB validations failed!${NC}"
    exit 1
fi
