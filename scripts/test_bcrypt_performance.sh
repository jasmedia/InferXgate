#!/bin/bash
# Test script for bcrypt cost 10 performance

set -e

GATEWAY_URL="http://localhost:3000"
TEST_EMAIL="test_$(date +%s)@example.com"
TEST_PASSWORD="TestPassword123!"

echo "=========================================="
echo "Bcrypt Cost 10 Performance Test"
echo "=========================================="
echo ""

# Step 1: Register a test user
echo "→ Registering test user..."
REGISTER_RESPONSE=$(curl -s -X POST "${GATEWAY_URL}/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${TEST_EMAIL}\",\"password\":\"${TEST_PASSWORD}\"}")

JWT_TOKEN=$(echo "$REGISTER_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

if [ -z "$JWT_TOKEN" ]; then
    echo "✗ Failed to register user"
    echo "Response: $REGISTER_RESPONSE"
    exit 1
fi

echo "✓ User registered successfully"
echo ""

# Step 2: Generate a virtual key
echo "→ Generating virtual key..."
KEY_RESPONSE=$(curl -s -X POST "${GATEWAY_URL}/auth/key/generate" \
  -H "Authorization: Bearer ${JWT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"name":"Test Key","max_budget":100.0}')

VIRTUAL_KEY=$(echo "$KEY_RESPONSE" | grep -o '"key":"[^"]*"' | cut -d'"' -f4)

if [ -z "$VIRTUAL_KEY" ]; then
    echo "✗ Failed to generate virtual key"
    echo "Response: $KEY_RESPONSE"
    exit 1
fi

echo "✓ Virtual key generated: ${VIRTUAL_KEY:0:20}..."
echo ""

# Step 3: Test performance with 5 requests
echo "→ Testing authentication performance (5 requests)..."
echo ""

REQUEST_PAYLOAD='{
  "model": "claude-sonnet-4-5-20250929",
  "messages": [{"role": "user", "content": "Say hello"}],
  "max_tokens": 10
}'

TIMES_FILE=$(mktemp)

for i in {1..5}; do
    # Use curl's time measurement
    RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME_TOTAL:%{time_total}" \
      -X POST "${GATEWAY_URL}/v1/chat/completions" \
      -H "Authorization: Bearer ${VIRTUAL_KEY}" \
      -H "Content-Type: application/json" \
      -d "$REQUEST_PAYLOAD")

    HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE:" | cut -d: -f2)
    TIME_SECONDS=$(echo "$RESPONSE" | grep "TIME_TOTAL:" | cut -d: -f2)

    if [ "$HTTP_CODE" = "200" ]; then
        # Convert to milliseconds
        TIME_MS=$(echo "$TIME_SECONDS * 1000 / 1" | bc)
        echo "  Request $i: ${TIME_MS}ms ✓"
        echo "$TIME_MS" >> "$TIMES_FILE"
    else
        echo "  Request $i: FAILED (HTTP $HTTP_CODE)"
        echo "  Response preview: $(echo "$RESPONSE" | head -3)"
    fi
done

echo ""
echo "=========================================="
echo "Results Summary"
echo "=========================================="

if [ -s "$TIMES_FILE" ]; then
    # Calculate statistics
    NUM_SUCCESS=$(wc -l < "$TIMES_FILE" | tr -d ' ')
    SUM=$(awk '{sum+=$1} END {print sum}' "$TIMES_FILE")
    AVG=$(echo "$SUM / $NUM_SUCCESS" | bc)
    MIN=$(sort -n "$TIMES_FILE" | head -1)
    MAX=$(sort -n "$TIMES_FILE" | tail -1)

    echo "Successful requests: ${NUM_SUCCESS}/5"
    echo "Average time: ${AVG}ms"
    echo "Min time: ${MIN}ms"
    echo "Max time: ${MAX}ms"
    echo ""

    if [ $AVG -lt 200 ]; then
        echo "✓ EXCELLENT! Authentication is fast (<200ms)"
        echo "  Bcrypt cost 10 is working as expected"
        echo "  This is approximately 45-90x faster than cost 15!"
    elif [ $AVG -lt 500 ]; then
        echo "✓ GOOD! Authentication is acceptable (<500ms)"
        echo "  Bcrypt cost 10 provides good security/performance balance"
    elif [ $AVG -lt 2000 ]; then
        echo "⚠ SLOW: Authentication is taking ${AVG}ms"
        echo "  This is better than before but still slow"
    else
        echo "✗ VERY SLOW: Authentication is taking ${AVG}ms"
        echo "  Bcrypt cost might still be too high"
    fi
else
    echo "✗ All requests failed!"
fi

# Cleanup
rm -f "$TIMES_FILE"

echo ""
echo "=========================================="
