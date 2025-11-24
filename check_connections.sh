#!/bin/bash

# Connection Check Script for LLM Gateway
# Checks Redis and PostgreSQL connectivity before starting the backend

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "==========================================="
echo "  LLM Gateway - Connection Check"
echo "==========================================="
echo ""

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}Warning: .env file not found${NC}"
    echo "Creating .env from .env.example..."
    if [ -f .env.example ]; then
        cp .env.example .env
        echo -e "${GREEN}.env file created${NC}"
    else
        echo -e "${RED}Error: .env.example not found${NC}"
        exit 1
    fi
fi

# Load environment variables
export $(cat .env | grep -v '^#' | xargs)

# Default values
REDIS_URL=${REDIS_URL:-redis://localhost:6379}
DATABASE_URL=${DATABASE_URL:-postgresql://inferxgate:inferxgate@localhost:5432/inferxgate}

# Extract connection details
REDIS_HOST=$(echo $REDIS_URL | sed -n 's/.*:\/\/\([^:]*\).*/\1/p')
REDIS_PORT=$(echo $REDIS_URL | sed -n 's/.*:\([0-9]*\)$/\1/p')
REDIS_HOST=${REDIS_HOST:-localhost}
REDIS_PORT=${REDIS_PORT:-6379}

PG_HOST=$(echo $DATABASE_URL | sed -n 's/.*@\([^:]*\).*/\1/p')
PG_PORT=$(echo $DATABASE_URL | sed -n 's/.*:\([0-9]*\)\/.*$/\1/p')
PG_HOST=${PG_HOST:-localhost}
PG_PORT=${PG_PORT:-5432}

echo "Checking connections..."
echo ""

# Function to check if a port is open
check_port() {
    local host=$1
    local port=$2
    nc -zv -w 2 $host $port 2>&1 > /dev/null
    return $?
}

# Check Redis
echo -n "Redis ($REDIS_HOST:$REDIS_PORT)... "
if check_port $REDIS_HOST $REDIS_PORT; then
    echo -e "${GREEN}✓ CONNECTED${NC}"
    REDIS_OK=1
else
    echo -e "${RED}✗ NOT CONNECTED${NC}"
    REDIS_OK=0
fi

# Check PostgreSQL
echo -n "PostgreSQL ($PG_HOST:$PG_PORT)... "
if check_port $PG_HOST $PG_PORT; then
    echo -e "${GREEN}✓ CONNECTED${NC}"
    PG_OK=1
else
    echo -e "${RED}✗ NOT CONNECTED${NC}"
    PG_OK=0
fi

echo ""
echo "==========================================="

# Summary
if [ $REDIS_OK -eq 1 ] && [ $PG_OK -eq 1 ]; then
    echo -e "${GREEN}All connections OK!${NC}"
    echo "You can now start the backend with: cd backend && cargo run"
    exit 0
else
    echo -e "${YELLOW}Some connections failed${NC}"
    echo ""
    echo "To start the required services, you have these options:"
    echo ""
    echo "1. Start with Docker Compose (recommended):"
    echo "   make docker-up"
    echo ""
    echo "2. Start individual containers:"
    if [ $REDIS_OK -eq 0 ]; then
        echo "   docker run -d -p 6379:6379 --name llm-gateway-redis redis:7-alpine"
    fi
    if [ $PG_OK -eq 0 ]; then
        echo "   docker run -d -p 5432:5432 --name inferxgate-postgres \\"
        echo "     -e POSTGRES_USER=inferxgate \\"
        echo "     -e POSTGRES_PASSWORD=inferxgate \\"
        echo "     -e POSTGRES_DB=inferxgate \\"
        echo "     postgres:18-alpine"
    fi
    echo ""
    echo "3. Or install and run locally (macOS):"
    if [ $REDIS_OK -eq 0 ]; then
        echo "   brew install redis && brew services start redis"
    fi
    if [ $PG_OK -eq 0 ]; then
        echo "   brew install postgresql && brew services start postgresql"
    fi
    echo ""
    exit 1
fi
