#!/usr/bin/env bash
# Test Ecowitt GW1100 HTTP POST format
# Usage: ./test_ecowitt_format.sh [http://host:port]

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

    TESTS_RUN=$((TESTS_RUN + 1))

    echo -e "\n${YELLOW}Test ${TESTS_RUN}: ${test_name}${NC}"
    echo "POST Data: ${post_data}"

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

    if [ "$status_code" -eq "$expected_status" ]; then
        echo -e "${GREEN}✓ PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAILED - Expected ${expected_status}, got ${status_code}${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Function to verify data in API
verify_current_api() {
    echo -e "\n${YELLOW}Verifying data in /api/v1/current${NC}"

    response=$(curl -s -w "\n%{http_code}" "${HOST}/api/v1/current" 2>&1)
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    echo "API Status: ${status_code}"
    echo "API Response: ${body}"

    if [ "$status_code" -eq 200 ]; then
        echo -e "${GREEN}✓ API accessible and returning data${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠ API returned status ${status_code}${NC}"
        return 1
    fi
}

echo "================================================"
echo "Ecowitt GW1100 HTTP POST Format Tests"
echo "Target: ${URL}"
echo "================================================"

# Test 1: Complete valid Ecowitt format
run_test "Complete Valid Ecowitt Format" \
    "stationtype=GW1100&baromabsin=29.92&baromrelin=30.01&tempf=78.6&humidity=52&winddir=180&windspeedmph=3.2&windgustmph=5.5&solarradiation=120.5&uv=2&dateutc=now&softwaretype=GW1100"

sleep 1

# Test 2: Minimal valid data
run_test "Minimal Valid Data" \
    "stationtype=GW1100&dateutc=now&tempf=72.0"

sleep 1

# Test 3: With timestamp in seconds
run_test "Timestamp in Seconds" \
    "stationtype=GW1100&dateutc=$(date +%s)&tempf=75.5&humidity=45"

sleep 1

# Test 4: All temperature and humidity sensors
run_test "Multiple Temperature Sensors" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&temp1f=68.5&temp2f=71.3&humidity=55&humidity1=48&humidity2=52"

sleep 1

# Test 5: Rain sensors
run_test "Rain Sensors" \
    "stationtype=GW1100&dateutc=now&tempf=65.0&rainin=0.05&dailyrainin=0.23&weeklyrainin=1.45&monthlyrainin=3.67&yearlyrainin=42.15"

sleep 1

# Test 6: Wind measurements
run_test "Wind Measurements" \
    "stationtype=GW1100&dateutc=now&tempf=70.0&winddir=245&windspeedmph=8.5&windgustmph=12.3&maxdailygust=15.7"

sleep 1

# Test 7: Solar and UV
run_test "Solar and UV" \
    "stationtype=GW1100&dateutc=now&tempf=80.0&solarradiation=850.5&uv=8"

sleep 1

# Test 8: All soil moisture sensors
run_test "Soil Moisture Sensors" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&soilmoisture1=45&soilmoisture2=52&soilmoisture3=38&soilmoisture4=41"

sleep 1

# Test 9: Battery levels
run_test "Battery Levels" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&wh65batt=0&batt1=0&batt2=1&soilbatt1=1.2&soilbatt2=1.3"

sleep 1

# Test 10: PM2.5 air quality
run_test "PM2.5 Air Quality" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&pm25=12.5&pm25_24h=15.3&pm25_aqi=42"

sleep 1

# Test 11: Lightning detection
run_test "Lightning Detection" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&lightning_num=5&lightning_distance=8&lightning_time=$(date +%s)"

sleep 1

# Test 12: Extreme values
run_test "Extreme Values" \
    "stationtype=GW1100&dateutc=now&tempf=-40.0&humidity=100&windspeedmph=150.0&baromabsin=35.00&baromrelin=25.00"

sleep 1

# Test 13: URL-encoded special characters
run_test "Special Characters" \
    "stationtype=GW1100%20Pro&dateutc=now&tempf=72.0&model=Test%2BStation&location=Home%20%26%20Garden"

sleep 1

# Test 14: Empty values (should still work)
run_test "Empty Optional Values" \
    "stationtype=GW1100&dateutc=now&tempf=72.0&humidity=&windspeedmph=&winddir="

sleep 1

# Test 15: Verify current API after all POSTs
verify_current_api

# Summary
echo ""
echo "================================================"
echo "Test Summary"
echo "================================================"
echo "Total Tests: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"

if [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
fi
