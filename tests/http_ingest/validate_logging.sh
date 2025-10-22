#!/usr/bin/env bash
# Validate logging for HTTP POST ingestion
# Usage: ./validate_logging.sh [log_file_or_container]

set -euo pipefail

# Configuration
LOG_SOURCE="${1:-container}"  # "container" or path to log file
CONTAINER_NAME="weerust-rust"

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

# Function to get logs
get_logs() {
    if [ "${LOG_SOURCE}" = "container" ]; then
        # Get logs from Docker container
        if docker ps -q -f name="${CONTAINER_NAME}" &> /dev/null; then
            docker logs "${CONTAINER_NAME}" 2>&1 | tail -n 500
        else
            echo "ERROR: Container ${CONTAINER_NAME} not found or not running"
            exit 1
        fi
    else
        # Read from log file
        if [ -f "${LOG_SOURCE}" ]; then
            tail -n 500 "${LOG_SOURCE}"
        else
            echo "ERROR: Log file ${LOG_SOURCE} not found"
            exit 1
        fi
    fi
}

# Function to run a log validation test
run_test() {
    local test_name="$1"
    local pattern="$2"
    local should_exist="${3:-true}"

    TESTS_RUN=$((TESTS_RUN + 1))

    echo -e "\n${YELLOW}Test ${TESTS_RUN}: ${test_name}${NC}"
    echo "Pattern: ${pattern}"

    count=$(get_logs | grep -c "${pattern}" || true)

    if [ "${should_exist}" = "true" ]; then
        if [ ${count} -gt 0 ]; then
            echo -e "${GREEN}✓ PASSED - Found ${count} occurrences${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            return 0
        else
            echo -e "${RED}✗ FAILED - Pattern not found${NC}"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            return 1
        fi
    else
        if [ ${count} -eq 0 ]; then
            echo -e "${GREEN}✓ PASSED - Pattern not found (as expected)${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            return 0
        else
            echo -e "${RED}✗ FAILED - Found ${count} occurrences (expected 0)${NC}"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            return 1
        fi
    fi
}

echo "================================================"
echo "Logging Validation Tests"
echo "================================================"
echo "Log Source: ${LOG_SOURCE}"
if [ "${LOG_SOURCE}" = "container" ]; then
    echo "Container: ${CONTAINER_NAME}"
fi
echo "================================================"

# Test 1: Application startup logs
run_test "Application Startup" \
    "Starting.*weewx\|Starting.*server\|Listening on" \
    "true"

# Test 2: HTTP server binding
run_test "HTTP Server Binding" \
    "0.0.0.0:8080\|listening.*8080\|bind.*8080" \
    "true"

# Test 3: Database connection
run_test "Database Connection" \
    "Connected to database\|MariaDB\|MySQL\|database.*connected" \
    "true"

# Test 4: POST request logging
run_test "POST Request Logging" \
    "POST /data\|POST.*ingest\|Received POST" \
    "true"

# Test 5: Data insertion logging (if INSERT_LOGGING=true)
if [ "${INSERT_LOGGING:-true}" = "true" ]; then
    run_test "Data Insertion Logging" \
        "INSERT\|Inserted.*record\|Writing.*data" \
        "true"
fi

# Test 6: No critical errors
run_test "No Critical Errors" \
    "CRITICAL\|FATAL\|panic" \
    "false"

# Test 7: No connection errors
run_test "No Connection Errors" \
    "Connection refused\|Cannot connect\|Connection timeout" \
    "false"

# Test 8: No SQL errors
run_test "No SQL Errors" \
    "SQL error\|Query failed\|Syntax error.*SQL" \
    "false"

# Detailed log analysis
echo -e "\n${BLUE}Detailed Log Analysis${NC}"
echo "================================================"

# Count log levels
echo -e "\n${BLUE}Log Level Distribution:${NC}"
for level in INFO WARN ERROR DEBUG TRACE; do
    count=$(get_logs | grep -c "\[${level}\]\|${level}:" || true)
    if [ ${count} -gt 0 ]; then
        case ${level} in
            INFO)
                echo -e "  ${GREEN}${level}: ${count}${NC}"
                ;;
            WARN)
                echo -e "  ${YELLOW}${level}: ${count}${NC}"
                ;;
            ERROR)
                echo -e "  ${RED}${level}: ${count}${NC}"
                ;;
            *)
                echo "  ${level}: ${count}"
                ;;
        esac
    fi
done

# Show recent errors if any
ERROR_COUNT=$(get_logs | grep -c "\[ERROR\]\|ERROR:" || true)
if [ ${ERROR_COUNT} -gt 0 ]; then
    echo -e "\n${RED}Recent Errors (last 5):${NC}"
    get_logs | grep "\[ERROR\]\|ERROR:" | tail -5
fi

# Show recent warnings if any
WARN_COUNT=$(get_logs | grep -c "\[WARN\]\|WARN:" || true)
if [ ${WARN_COUNT} -gt 0 ]; then
    echo -e "\n${YELLOW}Recent Warnings (last 5):${NC}"
    get_logs | grep "\[WARN\]\|WARN:" | tail -5
fi

# Check for performance metrics in logs
echo -e "\n${BLUE}Performance Metrics in Logs:${NC}"
TIMING_LOGS=$(get_logs | grep -i "ms\|milliseconds\|duration\|took" || true)
if [ -n "${TIMING_LOGS}" ]; then
    echo "${TIMING_LOGS}" | tail -5
else
    echo "  No performance timing logs found"
fi

# Check for specific weather data logging
echo -e "\n${BLUE}Weather Data in Logs (sample):${NC}"
WEATHER_LOGS=$(get_logs | grep -i "tempf\|temperature\|humidity\|wind" || true)
if [ -n "${WEATHER_LOGS}" ]; then
    echo "${WEATHER_LOGS}" | tail -3
else
    echo "  No weather data logs found"
fi

# Log file rotation check (if using files)
if [ "${LOG_SOURCE}" != "container" ] && [ -f "${LOG_SOURCE}" ]; then
    echo -e "\n${BLUE}Log File Statistics:${NC}"
    FILE_SIZE=$(ls -lh "${LOG_SOURCE}" | awk '{print $5}')
    LINE_COUNT=$(wc -l < "${LOG_SOURCE}")
    echo "  File size: ${FILE_SIZE}"
    echo "  Line count: ${LINE_COUNT}"

    if [ ${LINE_COUNT} -gt 100000 ]; then
        echo -e "  ${YELLOW}⚠ Log file is very large, consider rotation${NC}"
    fi
fi

# Check log timestamps
echo -e "\n${BLUE}Log Timestamp Analysis:${NC}"
FIRST_LOG=$(get_logs | head -1)
LAST_LOG=$(get_logs | tail -1)
echo "  First log entry: ${FIRST_LOG:0:80}..."
echo "  Last log entry: ${LAST_LOG:0:80}..."

# Check for continuous logging (no gaps > 5 minutes)
echo -e "\n${BLUE}Checking for logging gaps...${NC}"
# This is a simple check - could be enhanced based on actual timestamp format
LOG_COUNT=$(get_logs | wc -l)
if [ ${LOG_COUNT} -lt 10 ]; then
    echo -e "  ${YELLOW}⚠ Very few log entries (${LOG_COUNT})${NC}"
else
    echo -e "  ${GREEN}✓ Active logging detected (${LOG_COUNT} entries)${NC}"
fi

# Summary
echo ""
echo "================================================"
echo "Logging Validation Summary"
echo "================================================"
echo "Total Tests: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
echo ""
echo "Log Statistics:"
echo "  Errors: ${ERROR_COUNT}"
echo "  Warnings: ${WARN_COUNT}"
echo "  Total log lines: ${LOG_COUNT}"

if [ ${TESTS_FAILED} -eq 0 ] && [ ${ERROR_COUNT} -eq 0 ]; then
    echo -e "\n${GREEN}✓ All logging validations passed with no errors!${NC}"
    exit 0
elif [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "\n${YELLOW}⚠ Logging validations passed but ${ERROR_COUNT} errors found in logs${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some logging validations failed!${NC}"
    exit 1
fi
