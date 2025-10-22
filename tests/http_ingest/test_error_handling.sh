#!/usr/bin/env bash
# Test error handling for HTTP POST endpoint
# Usage: ./test_error_handling.sh [http://host:port]

set -euo pipefail

# Configuration
HOST="${1:-http://localhost:8080}"
ENDPOINT="/data"
URL="${HOST}${ENDPOINT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local post_data="$2"
    local expected_status="${3:-200}"
    local should_accept="${4:-true}"  # Whether server should accept request

    TESTS_RUN=$((TESTS_RUN + 1))

    echo -e "\n${YELLOW}Test ${TESTS_RUN}: ${test_name}${NC}"
    echo "POST Data: ${post_data}"
    echo "Expected: Should $([ "$should_accept" = "true" ] && echo "accept" || echo "reject")"

    # Make the request and capture response
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Content-Type: application/x-www-form-urlencoded" \
        --data "${post_data}" \
        "${URL}" 2>&1)

    # Extract status code (last line)
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    echo "Response Status: ${status_code}"
    echo "Response Body: ${body}"

    # For error tests, we accept either graceful handling (200) or proper error (400/422)
    if [ "$should_accept" = "true" ]; then
        if [ "$status_code" -eq 200 ]; then
            echo -e "${GREEN}✓ PASSED - Server gracefully handled request${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            return 0
        else
            echo -e "${RED}✗ FAILED - Server rejected valid request with ${status_code}${NC}"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            return 1
        fi
    else
        if [ "$status_code" -eq 200 ] || [ "$status_code" -eq 400 ] || [ "$status_code" -eq 422 ]; then
            echo -e "${GREEN}✓ PASSED - Server handled gracefully (${status_code})${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            return 0
        else
            echo -e "${RED}✗ FAILED - Unexpected status ${status_code}${NC}"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            return 1
        fi
    fi
}

echo "================================================"
echo "HTTP POST Error Handling Tests"
echo "Target: ${URL}"
echo "================================================"

# Test 1: Completely empty POST
run_test "Empty POST Body" \
    "" \
    200 \
    "false"

sleep 1

# Test 2: Invalid key-value format
run_test "Malformed Data - No Equals Sign" \
    "just_random_text_without_structure" \
    200 \
    "false"

sleep 1

# Test 3: Missing dateutc field
run_test "Missing dateutc Field" \
    "stationtype=GW1100&tempf=72.0&humidity=50" \
    200 \
    "true"

sleep 1

# Test 4: Invalid temperature (not a number)
run_test "Invalid Temperature Type" \
    "stationtype=GW1100&dateutc=now&tempf=NOT_A_NUMBER&humidity=50" \
    200 \
    "false"

sleep 1

# Test 5: Invalid humidity (out of range)
run_test "Invalid Humidity Range" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&humidity=150" \
    200 \
    "false"

sleep 1

# Test 6: Invalid wind direction (out of range)
run_test "Invalid Wind Direction" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&winddir=400" \
    200 \
    "false"

sleep 1

# Test 7: Negative humidity
run_test "Negative Humidity" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&humidity=-10" \
    200 \
    "false"

sleep 1

# Test 8: Multiple values for same field
run_test "Duplicate Field Names" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&tempf=80.0&humidity=50" \
    200 \
    "true"

sleep 1

# Test 9: SQL injection attempt
run_test "SQL Injection Attempt" \
    "stationtype=GW1100'; DROP TABLE weather; --&dateutc=now&tempf=72.0" \
    200 \
    "true"

sleep 1

# Test 10: XSS attempt
run_test "XSS Attempt" \
    "stationtype=<script>alert('XSS')</script>&dateutc=now&tempf=72.0" \
    200 \
    "true"

sleep 1

# Test 11: Extremely long field value
run_test "Extremely Long Field Value" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&custom_field=$(python3 -c 'print("A"*10000)')" \
    200 \
    "false"

sleep 1

# Test 12: Unicode characters
run_test "Unicode Characters" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&location=测试站点" \
    200 \
    "true"

sleep 1

# Test 13: Null bytes
run_test "Null Bytes in Data" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&field=$(printf 'test\x00data')" \
    200 \
    "false"

sleep 1

# Test 14: Missing station type
run_test "Missing Station Type" \
    "dateutc=now&tempf=72.0&humidity=50" \
    200 \
    "true"

sleep 1

# Test 15: Invalid date format
run_test "Invalid Date Format" \
    "stationtype=GW1100&dateutc=INVALID_DATE&tempf=72.0" \
    200 \
    "false"

sleep 1

# Test 16: Binary data
run_test "Binary Data" \
    "$(printf '\x00\x01\x02\x03\x04\x05')" \
    200 \
    "false"

sleep 1

# Test 17: Very large numeric value
run_test "Overflow Numeric Value" \
    "stationtype=GW1100&dateutc=now&tempf=999999999999999999999.0" \
    200 \
    "false"

sleep 1

# Test 18: Wrong content type
echo -e "\n${YELLOW}Test Special: Wrong Content-Type Header${NC}"
TESTS_RUN=$((TESTS_RUN + 1))
response=$(curl -s -w "\n%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    --data '{"stationtype":"GW1100","tempf":72.0}' \
    "${URL}" 2>&1)
status_code=$(echo "$response" | tail -n1)
if [ "$status_code" -eq 200 ] || [ "$status_code" -eq 400 ] || [ "$status_code" -eq 415 ]; then
    echo -e "${GREEN}✓ PASSED - Server handled wrong content-type gracefully${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}✗ FAILED - Unexpected status ${status_code}${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

sleep 1

# Test 19: GET request to POST endpoint
echo -e "\n${YELLOW}Test Special: GET Request to POST Endpoint${NC}"
TESTS_RUN=$((TESTS_RUN + 1))
response=$(curl -s -w "\n%{http_code}" -X GET "${URL}?stationtype=GW1100&tempf=72.0" 2>&1)
status_code=$(echo "$response" | tail -n1)
if [ "$status_code" -eq 200 ] || [ "$status_code" -eq 405 ]; then
    echo -e "${GREEN}✓ PASSED - Server handled GET request appropriately${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}✗ FAILED - Unexpected status ${status_code}${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Summary
echo ""
echo "================================================"
echo "Test Summary"
echo "================================================"
echo "Total Tests: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"

if [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "\n${GREEN}All error handling tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some error handling tests failed!${NC}"
    exit 1
fi
