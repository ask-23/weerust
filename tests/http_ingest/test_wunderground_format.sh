#!/usr/bin/env bash
# Test Weather Underground HTTP POST format
# Usage: ./test_wunderground_format.sh [http://host:port]

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

echo "================================================"
echo "Weather Underground HTTP POST Format Tests"
echo "Target: ${URL}"
echo "================================================"

# Test 1: Complete valid WU format
run_test "Complete Valid WU Format" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=72.5&baromin=29.92&humidity=55&windspeedmph=5.0&windgustmph=7.0&winddir=180&dewptf=56.3&rainin=0.00&dailyrainin=0.05&solarradiation=85.2&UV=1&softwaretype=WeatherUnderground"

sleep 1

# Test 2: Minimal WU format
run_test "Minimal WU Format" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=72.5"

sleep 1

# Test 3: WU with action=updateraw
run_test "WU with action=updateraw" \
    "ID=STATION123&PASSWORD=mypass&action=updateraw&dateutc=now&tempf=75.0&humidity=60"

sleep 1

# Test 4: WU realtime parameter
run_test "WU Realtime Parameter" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=70.0&realtime=1&rtfreq=5"

sleep 1

# Test 5: WU with timestamp
run_test "WU with Timestamp" \
    "ID=STATION123&PASSWORD=mypass&dateutc=$(date -u +%Y-%m-%d+%H:%M:%S)&tempf=73.5"

sleep 1

# Test 6: WU imperial units
run_test "WU Imperial Units" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=72.0&baromin=30.12&windspeedmph=8.5&rainin=0.10"

sleep 1

# Test 7: WU metric conversion (should work)
run_test "WU with Celsius" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempc=22.0&baromhpa=1013.25"

sleep 1

# Test 8: WU rain accumulation
run_test "WU Rain Accumulation" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=65.0&rainin=0.05&dailyrainin=0.23&monthlyrainin=3.67&yearlyrainin=42.15"

sleep 1

# Test 9: WU wind chill and heat index
run_test "WU Wind Chill and Heat Index" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=95.0&humidity=70&windchillf=32.0&heatindexf=105.0"

sleep 1

# Test 10: WU indoor measurements
run_test "WU Indoor Measurements" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=72.0&indoortempf=74.5&indoorhumidity=45"

sleep 1

# Test 11: WU soil temperature
run_test "WU Soil Temperature" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=72.0&soiltempf=68.5&soilmoisture=42"

sleep 1

# Test 12: WU AQI parameters
run_test "WU AQI Parameters" \
    "ID=STATION123&PASSWORD=mypass&dateutc=now&tempf=72.0&AqNO=5&AqNO2=8&AqNO2T=12&AqNOX=15&AqNOXT=18"

sleep 1

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
