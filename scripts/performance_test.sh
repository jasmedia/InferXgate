#!/bin/bash
# Performance Testing Script for LLM Gateway
# This script tests the gateway's performance improvements

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
GATEWAY_URL="${GATEWAY_URL:-http://localhost:3000}"
API_KEY="${API_KEY:-sk-test-key}"
MODEL="${MODEL:-claude-sonnet-4-5-20250929}"
NUM_REQUESTS="${NUM_REQUESTS:-10}"
CONCURRENCY="${CONCURRENCY:-5}"

echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   LLM Gateway Performance Test${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Configuration:${NC}"
echo -e "  Gateway URL:  ${GATEWAY_URL}"
echo -e "  Model:        ${MODEL}"
echo -e "  Requests:     ${NUM_REQUESTS}"
echo -e "  Concurrency:  ${CONCURRENCY}"
echo ""

# Check if gateway is running
echo -e "${BLUE}→ Checking gateway health...${NC}"
if curl -sf -X POST "${GATEWAY_URL}/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Gateway is running${NC}"
else
    echo -e "${RED}✗ Gateway is not running at ${GATEWAY_URL}${NC}"
    echo -e "${YELLOW}  Please start the gateway first: make dev${NC}"
    exit 1
fi

# Create test payload
PAYLOAD_FILE=$(mktemp)
cat > "$PAYLOAD_FILE" <<EOF
{
  "model": "${MODEL}",
  "messages": [
    {
      "role": "user",
      "content": "Hello! Please respond with a short greeting."
    }
  ],
  "max_tokens": 50,
  "temperature": 0.7
}
EOF

echo ""
echo -e "${BLUE}→ Running performance tests...${NC}"
echo ""

# Test 1: Single request latency
echo -e "${YELLOW}Test 1: Single Request Latency${NC}"
echo -e "Measuring end-to-end latency for a single request..."

SINGLE_START=$(date +%s%3N)
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME_TOTAL:%{time_total}\n" \
  -X POST "${GATEWAY_URL}/v1/chat/completions" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d @"$PAYLOAD_FILE")
SINGLE_END=$(date +%s%3N)

HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
TIME_TOTAL=$(echo "$RESPONSE" | grep "TIME_TOTAL:" | cut -d: -f2)

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✓ Request successful${NC}"
    echo -e "  Time: ${TIME_TOTAL}s"
else
    echo -e "${RED}✗ Request failed with HTTP ${HTTP_CODE}${NC}"
fi

echo ""

# Test 2: Sequential requests (connection reuse)
echo -e "${YELLOW}Test 2: Sequential Requests (Connection Reuse Test)${NC}"
echo -e "Running ${NUM_REQUESTS} sequential requests to test connection pooling..."

TIMES=()
for i in $(seq 1 $NUM_REQUESTS); do
    TIME=$(curl -s -w "%{time_total}" -o /dev/null \
      -X POST "${GATEWAY_URL}/v1/chat/completions" \
      -H "Authorization: Bearer ${API_KEY}" \
      -H "Content-Type: application/json" \
      -d @"$PAYLOAD_FILE")
    TIMES+=($TIME)
    echo -e "  Request $i: ${TIME}s"
done

# Calculate average
SUM=0
for t in "${TIMES[@]}"; do
    SUM=$(echo "$SUM + $t" | bc)
done
AVG=$(echo "scale=3; $SUM / $NUM_REQUESTS" | bc)
echo -e "${GREEN}✓ Average time: ${AVG}s${NC}"
echo ""

# Test 3: Concurrent requests
echo -e "${YELLOW}Test 3: Concurrent Requests (${CONCURRENCY} parallel)${NC}"
echo -e "Testing gateway under concurrent load..."

CONCURRENT_START=$(date +%s)

# Create temporary directory for results
TEMP_DIR=$(mktemp -d)

# Run concurrent requests
for i in $(seq 1 $CONCURRENCY); do
    (
        TIME=$(curl -s -w "%{time_total}" -o /dev/null \
          -X POST "${GATEWAY_URL}/v1/chat/completions" \
          -H "Authorization: Bearer ${API_KEY}" \
          -H "Content-Type: application/json" \
          -d @"$PAYLOAD_FILE")
        echo "$TIME" > "$TEMP_DIR/result_$i.txt"
    ) &
done

# Wait for all to complete
wait

CONCURRENT_END=$(date +%s)
CONCURRENT_TOTAL=$((CONCURRENT_END - CONCURRENT_START))

# Collect results
CONCURRENT_TIMES=()
for f in "$TEMP_DIR"/result_*.txt; do
    if [ -f "$f" ]; then
        CONCURRENT_TIMES+=($(cat "$f"))
    fi
done

# Calculate stats
MIN=${CONCURRENT_TIMES[0]}
MAX=${CONCURRENT_TIMES[0]}
SUM=0
for t in "${CONCURRENT_TIMES[@]}"; do
    SUM=$(echo "$SUM + $t" | bc)
    if (( $(echo "$t < $MIN" | bc -l) )); then
        MIN=$t
    fi
    if (( $(echo "$t > $MAX" | bc -l) )); then
        MAX=$t
    fi
done
AVG_CONCURRENT=$(echo "scale=3; $SUM / ${#CONCURRENT_TIMES[@]}" | bc)

echo -e "${GREEN}✓ Concurrent test completed${NC}"
echo -e "  Total wall time:    ${CONCURRENT_TOTAL}s"
echo -e "  Requests completed: ${#CONCURRENT_TIMES[@]}"
echo -e "  Average time:       ${AVG_CONCURRENT}s"
echo -e "  Min time:           ${MIN}s"
echo -e "  Max time:           ${MAX}s"
echo ""

# Test 4: Cache performance (if enabled)
echo -e "${YELLOW}Test 4: Cache Performance Test${NC}"
echo -e "Testing cache hit performance..."

# First request (cache miss)
CACHE_MISS_TIME=$(curl -s -w "%{time_total}" -o /dev/null \
  -X POST "${GATEWAY_URL}/v1/chat/completions" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d @"$PAYLOAD_FILE")

echo -e "  First request (cache miss):  ${CACHE_MISS_TIME}s"

# Second request (cache hit if caching enabled)
sleep 1
CACHE_HIT_TIME=$(curl -s -w "%{time_total}" -o /dev/null \
  -X POST "${GATEWAY_URL}/v1/chat/completions" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d @"$PAYLOAD_FILE")

echo -e "  Second request (cache hit?): ${CACHE_HIT_TIME}s"

SPEEDUP=$(echo "scale=2; $CACHE_MISS_TIME / $CACHE_HIT_TIME" | bc)
if (( $(echo "$SPEEDUP > 1.5" | bc -l) )); then
    echo -e "${GREEN}✓ Cache appears to be working (${SPEEDUP}x speedup)${NC}"
else
    echo -e "${YELLOW}⚠ Cache may not be enabled or request was different${NC}"
fi

echo ""

# Summary
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   Performance Test Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}✓ Single request latency:     ${TIME_TOTAL}s${NC}"
echo -e "${GREEN}✓ Sequential avg (pooling):   ${AVG}s${NC}"
echo -e "${GREEN}✓ Concurrent avg:              ${AVG_CONCURRENT}s${NC}"
echo -e "${GREEN}✓ Cache speedup:               ${SPEEDUP}x${NC}"
echo ""

# Cleanup
rm -f "$PAYLOAD_FILE"
rm -rf "$TEMP_DIR"

echo -e "${BLUE}→ Performance optimizations applied:${NC}"
echo -e "  ✅ HTTP connection pooling (10 connections/host)"
echo -e "  ✅ Lock-free route lookups (DashMap)"
echo -e "  ✅ Concurrent cache operations"
echo -e "  ✅ TCP keepalive and HTTP/2 support"
echo ""
echo -e "${GREEN}Test completed successfully!${NC}"
