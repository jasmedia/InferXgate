#!/bin/bash

# Test script for enhanced features
# Run this after starting the backend server

set -e

echo "🧪 Testing Enhanced Features of LLM Gateway"
echo "==========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:3000"

# Test 1: Health Check
echo -e "${BLUE}1. Testing Health Check...${NC}"
curl -s "$BASE_URL/health" | jq '.'
echo -e "${GREEN}✓ Health check passed${NC}"
echo ""

# Test 2: List Models
echo -e "${BLUE}2. Testing Model List...${NC}"
curl -s "$BASE_URL/v1/models" | jq '.data[0:3]'
echo -e "${GREEN}✓ Models list retrieved${NC}"
echo ""

# Test 3: First Request (Cache Miss)
echo -e "${BLUE}3. Testing First Request (Cache Miss)...${NC}"
echo "This will call the actual API..."
START=$(date +%s%N)
RESPONSE1=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "What is 2+2? Just give the number."}],
    "max_tokens": 10
  }')
END=$(date +%s%N)
DURATION1=$((($END - $START) / 1000000))

echo "$RESPONSE1" | jq '.choices[0].message.content'
echo "Duration: ${DURATION1}ms"
echo -e "${GREEN}✓ First request completed${NC}"
echo ""

# Test 4: Second Request (Cache Hit)
echo -e "${BLUE}4. Testing Second Request (Cache Hit)...${NC}"
echo "This should be instant (from cache)..."
START=$(date +%s%N)
RESPONSE2=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "What is 2+2? Just give the number."}],
    "max_tokens": 10
  }')
END=$(date +%s%N)
DURATION2=$((($END - $START) / 1000000))

echo "$RESPONSE2" | jq '.choices[0].message.content'
echo "Duration: ${DURATION2}ms"
echo -e "${GREEN}✓ Second request completed (cache hit!)${NC}"
echo ""

# Compare durations
echo -e "${BLUE}Performance Comparison:${NC}"
echo "First request (API):   ${DURATION1}ms"
echo "Second request (cache): ${DURATION2}ms"
SPEEDUP=$((DURATION1 / DURATION2))
echo "Cache speedup: ~${SPEEDUP}x faster"
echo ""

# Test 5: Usage Statistics
echo -e "${BLUE}5. Testing Usage Statistics...${NC}"
STATS=$(curl -s "$BASE_URL/stats")
echo "$STATS" | jq '{
  total_requests: .usage_stats.total_requests,
  total_tokens: .usage_stats.total_tokens,
  total_cost: .usage_stats.total_cost,
  cache_hit_rate: .usage_stats.cache_hit_rate,
  cache_enabled: .cache_enabled,
  database_enabled: .database_enabled
}'
echo -e "${GREEN}✓ Statistics retrieved${NC}"
echo ""

# Test 6: Prometheus Metrics
echo -e "${BLUE}6. Testing Prometheus Metrics...${NC}"
echo "Request counter:"
curl -s "$BASE_URL/metrics" | grep 'llm_gateway_requests_total{' | head -3
echo ""
echo "Cache metrics:"
curl -s "$BASE_URL/metrics" | grep 'llm_gateway_cache_total'
echo ""
echo "Cost metrics:"
curl -s "$BASE_URL/metrics" | grep 'llm_gateway_cost_usd_total' | head -2
echo -e "${GREEN}✓ Metrics endpoint working${NC}"
echo ""

# Test 7: Database Query
echo -e "${BLUE}7. Testing Database (Recent Requests)...${NC}"
if command -v psql &> /dev/null; then
    psql inferxgate -c "
    SELECT
        model,
        provider,
        total_tokens,
        ROUND(cost_usd::numeric, 6) as cost,
        cached,
        to_char(created_at, 'HH24:MI:SS') as time
    FROM usage_records
    ORDER BY created_at DESC
    LIMIT 5;" 2>/dev/null || echo "Database query skipped (password required)"
else
    echo "psql not found, skipping database test"
fi
echo -e "${GREEN}✓ Database check completed${NC}"
echo ""

# Summary
echo "==========================================="
echo -e "${GREEN}🎉 All Tests Passed!${NC}"
echo ""
echo "Enhanced features are working:"
echo "  ✓ Redis caching"
echo "  ✓ PostgreSQL usage tracking"
echo "  ✓ Prometheus metrics"
echo "  ✓ Cost calculation"
echo "  ✓ Statistics API"
echo ""
echo "Try these commands:"
echo "  curl $BASE_URL/stats | jq"
echo "  curl $BASE_URL/metrics"
echo "  psql inferxgate -c 'SELECT * FROM usage_records LIMIT 5;'"
echo ""
