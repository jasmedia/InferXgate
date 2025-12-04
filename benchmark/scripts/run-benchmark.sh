#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARK_DIR="$(dirname "$SCRIPT_DIR")"
K6_DIR="$BENCHMARK_DIR/k6"
TEST_TYPE="${1:-all}"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}InferXgate vs LiteLLM Benchmark Suite${NC}"
echo -e "${BLUE}========================================${NC}"

# Load environment
if [ -f "$BENCHMARK_DIR/.env.benchmark" ]; then
    echo -e "${GREEN}Loading environment from .env.benchmark${NC}"
    set -a
    source "$BENCHMARK_DIR/.env.benchmark"
    set +a
fi

# Check prerequisites
check_prerequisites() {
    echo -e "\n${YELLOW}Checking prerequisites...${NC}"

    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Error: Docker is not installed${NC}"
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} Docker installed"

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo -e "${RED}Error: Docker Compose is not installed${NC}"
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} Docker Compose installed"

    if ! command -v k6 &> /dev/null; then
        echo -e "${RED}Error: k6 is not installed${NC}"
        echo -e "${YELLOW}Install with: brew install k6${NC}"
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} k6 installed"

    if [ -z "$ANTHROPIC_API_KEY" ] || [ "$ANTHROPIC_API_KEY" = "your_anthropic_api_key_here" ]; then
        echo -e "${RED}Error: ANTHROPIC_API_KEY is not set${NC}"
        echo -e "${YELLOW}Set it in $BENCHMARK_DIR/.env.benchmark${NC}"
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} ANTHROPIC_API_KEY is set"

    echo -e "${GREEN}All prerequisites met!${NC}"
}

# Start services
start_services() {
    echo -e "\n${YELLOW}Starting Docker services...${NC}"
    cd "$BENCHMARK_DIR"

    # Build and start services
    docker compose -f docker-compose.benchmark.yml up -d --build

    echo -e "\n${YELLOW}Waiting for services to be healthy...${NC}"

    # Wait for InferXgate
    echo -n "  Waiting for InferXgate"
    for i in {1..60}; do
        if curl -sf http://localhost:3000/health > /dev/null 2>&1; then
            echo -e " ${GREEN}✓${NC}"
            break
        fi
        echo -n "."
        sleep 2
    done

    # Wait for LiteLLM
    echo -n "  Waiting for LiteLLM"
    for i in {1..90}; do
        if curl -sf http://localhost:4000/health/liveliness > /dev/null 2>&1; then
            echo -e " ${GREEN}✓${NC}"
            break
        fi
        echo -n "."
        sleep 2
    done

    echo -e "${GREEN}All services are ready!${NC}"
}

# Stop services
stop_services() {
    echo -e "\n${YELLOW}Stopping Docker services...${NC}"
    cd "$BENCHMARK_DIR"
    docker compose -f docker-compose.benchmark.yml down
    echo -e "${GREEN}Services stopped${NC}"
}

# Run k6 test
run_test() {
    local test_name=$1
    local target=$2

    echo -e "\n${BLUE}Running ${test_name} test against ${target}...${NC}"

    cd "$K6_DIR"
    mkdir -p results

    k6 run \
        -e TARGET="$target" \
        -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
        -e LITELLM_MASTER_KEY="${LITELLM_MASTER_KEY:-sk-benchmark-litellm-key}" \
        -e TEST_MODEL="${TEST_MODEL:-claude-sonnet-4-5-20250929}" \
        -e TEST_MAX_TOKENS="${TEST_MAX_TOKENS:-100}" \
        -e TEST_TEMPERATURE="${TEST_TEMPERATURE:-0.7}" \
        -e RESULTS_DIR="$K6_DIR/results" \
        "scripts/${test_name}.js"

    echo -e "${GREEN}Test completed!${NC}"
}

# Run benchmark suite
run_benchmark_suite() {
    local test_type=$1

    case $test_type in
        baseline)
            echo -e "\n${BLUE}=== BASELINE LATENCY TESTS ===${NC}"
            run_test "baseline-latency" "inferxgate"
            echo -e "\n${YELLOW}Cooling down for 30 seconds...${NC}"
            sleep 30
            run_test "baseline-latency" "litellm"
            ;;
        throughput)
            echo -e "\n${BLUE}=== THROUGHPUT RAMP TESTS ===${NC}"
            run_test "throughput-ramp" "inferxgate"
            echo -e "\n${YELLOW}Cooling down for 60 seconds...${NC}"
            sleep 60
            run_test "throughput-ramp" "litellm"
            ;;
        sustained)
            echo -e "\n${BLUE}=== SUSTAINED LOAD TESTS ===${NC}"
            run_test "sustained-load" "inferxgate"
            echo -e "\n${YELLOW}Cooling down for 60 seconds...${NC}"
            sleep 60
            run_test "sustained-load" "litellm"
            ;;
        all)
            run_benchmark_suite "baseline"
            echo -e "\n${YELLOW}Cooling down for 30 seconds before next test suite...${NC}"
            sleep 30
            run_benchmark_suite "throughput"
            echo -e "\n${YELLOW}Cooling down for 30 seconds before next test suite...${NC}"
            sleep 30
            run_benchmark_suite "sustained"
            ;;
        *)
            echo -e "${RED}Unknown test type: $test_type${NC}"
            echo -e "Usage: $0 [baseline|throughput|sustained|all]"
            exit 1
            ;;
    esac
}

# Print usage
print_usage() {
    echo -e "\nUsage: $0 [command]"
    echo -e "\nCommands:"
    echo -e "  ${GREEN}baseline${NC}    - Run baseline latency tests only"
    echo -e "  ${GREEN}throughput${NC}  - Run throughput ramp tests only"
    echo -e "  ${GREEN}sustained${NC}   - Run sustained load tests only"
    echo -e "  ${GREEN}all${NC}         - Run all tests (default)"
    echo -e "  ${GREEN}start${NC}       - Start Docker services only"
    echo -e "  ${GREEN}stop${NC}        - Stop Docker services"
    echo -e "  ${GREEN}help${NC}        - Show this help message"
    echo -e "\nResults are saved to: $K6_DIR/results/"
}

# Main
main() {
    case $TEST_TYPE in
        help|--help|-h)
            print_usage
            exit 0
            ;;
        start)
            check_prerequisites
            start_services
            exit 0
            ;;
        stop)
            stop_services
            exit 0
            ;;
        baseline|throughput|sustained|all)
            check_prerequisites
            start_services
            run_benchmark_suite "$TEST_TYPE"

            echo -e "\n${GREEN}========================================${NC}"
            echo -e "${GREEN}Benchmark Complete!${NC}"
            echo -e "${GREEN}========================================${NC}"
            echo -e "\nResults saved to: ${BLUE}$K6_DIR/results/${NC}"
            echo -e "Prometheus: ${BLUE}http://localhost:9090${NC}"
            echo -e "cAdvisor: ${BLUE}http://localhost:8080${NC}"
            echo -e "\nTo stop services: ${YELLOW}$0 stop${NC}"
            ;;
        *)
            echo -e "${RED}Unknown command: $TEST_TYPE${NC}"
            print_usage
            exit 1
            ;;
    esac
}

main
