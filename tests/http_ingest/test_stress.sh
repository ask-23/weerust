#!/usr/bin/env bash
# Stress test for HTTP POST endpoint - concurrent requests
# Usage: ./test_stress.sh [http://host:port] [num_requests] [concurrency]

set -euo pipefail

# Configuration
HOST="${1:-http://localhost:8080}"
NUM_REQUESTS="${2:-100}"
CONCURRENCY="${3:-10}"
ENDPOINT="/data"
URL="${HOST}${ENDPOINT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Statistics
TOTAL_REQUESTS=0
SUCCESSFUL_REQUESTS=0
FAILED_REQUESTS=0
START_TIME=$(date +%s)

echo "================================================"
echo "HTTP POST Stress Test"
echo "================================================"
echo "Target: ${URL}"
echo "Total Requests: ${NUM_REQUESTS}"
echo "Concurrency: ${CONCURRENCY}"
echo "================================================"

# Function to send a single request
send_request() {
    local request_id=$1
    local temp=$(echo "scale=2; 60 + ($RANDOM % 40)" | bc)
    local humidity=$(( 30 + RANDOM % 70 ))
    local windspeed=$(echo "scale=2; ($RANDOM % 50)" | bc)

    local post_data="stationtype=GW1100&dateutc=now&tempf=${temp}&humidity=${humidity}&windspeedmph=${windspeed}&request_id=${request_id}"

    local status_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/x-www-form-urlencoded" \
        --data "${post_data}" \
        "${URL}" 2>/dev/null)

    echo "${request_id},${status_code}"
}

export -f send_request
export URL

# Create temporary results file
RESULTS_FILE=$(mktemp)

echo -e "\n${BLUE}Starting stress test...${NC}\n"

# Run requests in parallel using GNU parallel or xargs
if command -v parallel &> /dev/null; then
    # Use GNU parallel if available
    seq 1 ${NUM_REQUESTS} | parallel -j ${CONCURRENCY} send_request > "${RESULTS_FILE}"
else
    # Fallback to xargs
    seq 1 ${NUM_REQUESTS} | xargs -P ${CONCURRENCY} -I {} bash -c "send_request {}" > "${RESULTS_FILE}"
fi

# Calculate statistics
TOTAL_REQUESTS=$(wc -l < "${RESULTS_FILE}" | tr -d ' ')
SUCCESSFUL_REQUESTS=$(grep -c ",200$" "${RESULTS_FILE}" || true)
FAILED_REQUESTS=$(grep -c -v ",200$" "${RESULTS_FILE}" || true)

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
REQUESTS_PER_SECOND=$(echo "scale=2; ${TOTAL_REQUESTS} / ${DURATION}" | bc)

# Display results
echo ""
echo "================================================"
echo "Stress Test Results"
echo "================================================"
echo "Duration: ${DURATION} seconds"
echo "Total Requests: ${TOTAL_REQUESTS}"
echo -e "${GREEN}Successful (200): ${SUCCESSFUL_REQUESTS}${NC}"
echo -e "${RED}Failed (!200): ${FAILED_REQUESTS}${NC}"
echo "Requests/Second: ${REQUESTS_PER_SECOND}"
echo "Success Rate: $(echo "scale=2; (${SUCCESSFUL_REQUESTS} * 100) / ${TOTAL_REQUESTS}" | bc)%"

# Status code breakdown
echo ""
echo "Status Code Breakdown:"
awk -F',' '{print $2}' "${RESULTS_FILE}" | sort | uniq -c | while read count code; do
    echo "  ${code}: ${count} requests"
done

# Performance test - measure response times
echo ""
echo "================================================"
echo "Response Time Test"
echo "================================================"

RESPONSE_TIMES_FILE=$(mktemp)

for i in {1..10}; do
    time_ms=$(curl -s -o /dev/null -w "%{time_total}" \
        -X POST \
        -H "Content-Type: application/x-www-form-urlencoded" \
        --data "stationtype=GW1100&dateutc=now&tempf=72.0&humidity=50&test_id=${i}" \
        "${URL}" 2>/dev/null | awk '{print $1 * 1000}')
    echo "${time_ms}" >> "${RESPONSE_TIMES_FILE}"
done

# Calculate average, min, max response times
if [ -s "${RESPONSE_TIMES_FILE}" ]; then
    AVG_TIME=$(awk '{ sum += $1; n++ } END { if (n > 0) print sum / n; }' "${RESPONSE_TIMES_FILE}")
    MIN_TIME=$(sort -n "${RESPONSE_TIMES_FILE}" | head -1)
    MAX_TIME=$(sort -n "${RESPONSE_TIMES_FILE}" | tail -1)

    echo "Average Response Time: ${AVG_TIME} ms"
    echo "Min Response Time: ${MIN_TIME} ms"
    echo "Max Response Time: ${MAX_TIME} ms"
fi

# Sustained load test
echo ""
echo "================================================"
echo "Sustained Load Test (30 seconds)"
echo "================================================"

SUSTAINED_START=$(date +%s)
SUSTAINED_END=$((SUSTAINED_START + 30))
SUSTAINED_COUNT=0
SUSTAINED_SUCCESS=0

while [ $(date +%s) -lt ${SUSTAINED_END} ]; do
    status=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/x-www-form-urlencoded" \
        --data "stationtype=GW1100&dateutc=now&tempf=72.0&humidity=50" \
        "${URL}" 2>/dev/null)

    SUSTAINED_COUNT=$((SUSTAINED_COUNT + 1))
    if [ "${status}" = "200" ]; then
        SUSTAINED_SUCCESS=$((SUSTAINED_SUCCESS + 1))
    fi

    # Brief sleep to avoid hammering too hard
    sleep 0.1
done

echo "Sustained requests: ${SUSTAINED_COUNT}"
echo "Sustained success: ${SUSTAINED_SUCCESS}"
echo "Sustained success rate: $(echo "scale=2; (${SUSTAINED_SUCCESS} * 100) / ${SUSTAINED_COUNT}" | bc)%"

# Cleanup
rm -f "${RESULTS_FILE}" "${RESPONSE_TIMES_FILE}"

# Final summary
echo ""
echo "================================================"
echo "Final Summary"
echo "================================================"

if [ ${FAILED_REQUESTS} -eq 0 ] && [ ${SUSTAINED_SUCCESS} -eq ${SUSTAINED_COUNT} ]; then
    echo -e "${GREEN}✓ All stress tests passed!${NC}"
    echo "  - 100% success rate on concurrent requests"
    echo "  - 100% success rate on sustained load"
    echo "  - Average response time: ${AVG_TIME} ms"
    exit 0
else
    echo -e "${YELLOW}⚠ Some issues detected:${NC}"
    if [ ${FAILED_REQUESTS} -gt 0 ]; then
        echo "  - ${FAILED_REQUESTS} failed requests in concurrent test"
    fi
    if [ ${SUSTAINED_SUCCESS} -ne ${SUSTAINED_COUNT} ]; then
        echo "  - $(( SUSTAINED_COUNT - SUSTAINED_SUCCESS )) failed requests in sustained load test"
    fi
    exit 1
fi
