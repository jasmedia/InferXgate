# InferXgate Benchmark Suite

Performance benchmarking suite comparing InferXgate against LiteLLM using k6 load testing.

## Prerequisites

- **Docker** and **Docker Compose**
- **k6** load testing tool
- **Anthropic API Key**

### Installing k6

```bash
# macOS
brew install k6

# Linux (Debian/Ubuntu)
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Windows
choco install k6
```

## Quick Start

1. **Configure your API key**

   Edit `.env.benchmark` and set your Anthropic API key:

   ```bash
   ANTHROPIC_API_KEY=sk-ant-your-key-here
   ```

2. **Run all benchmarks**

   ```bash
   ./scripts/run-benchmark.sh all
   ```

   This will:
   - Start Docker services (InferXgate, LiteLLM, Prometheus, etc.)
   - Run all test suites against both gateways
   - Save results to `k6/results/`

## Available Commands

```bash
# Run all benchmark suites
./scripts/run-benchmark.sh all

# Run specific test suite
./scripts/run-benchmark.sh baseline     # Baseline latency tests
./scripts/run-benchmark.sh throughput   # Throughput ramp tests
./scripts/run-benchmark.sh sustained    # Sustained load tests

# Service management
./scripts/run-benchmark.sh start        # Start Docker services only
./scripts/run-benchmark.sh stop         # Stop Docker services

# Help
./scripts/run-benchmark.sh help
```

## Test Suites

### 1. Baseline Latency (`baseline`)

Measures single-request latency with no concurrency.

- **VUs**: 1
- **Iterations**: 30
- **Purpose**: Establish baseline performance without load

### 2. Throughput Ramp (`throughput`)

Gradually increases load to find throughput limits.

- **Stages**:
  - 30s ramp to 5 VUs
  - 1m at 10 VUs
  - 1m at 20 VUs
  - 1m at 30 VUs
  - 1m at 50 VUs
  - 30s hold at 50 VUs
  - 30s ramp down
- **Purpose**: Identify maximum throughput and breaking points

### 3. Sustained Load (`sustained`)

Maintains constant load over extended period.

- **VUs**: 10
- **Duration**: 10 minutes
- **Purpose**: Test stability and performance consistency

## Configuration

### Environment Variables (`.env.benchmark`)

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_API_KEY` | Your Anthropic API key | *required* |
| `LITELLM_MASTER_KEY` | LiteLLM authentication key | `sk-benchmark-litellm-key` |
| `TEST_MODEL` | Model to use for tests | `claude-sonnet-4-5-20250929` |
| `TEST_MAX_TOKENS` | Max tokens per response | `100` |
| `TEST_TEMPERATURE` | Temperature setting | `0.7` |

### Resource Limits

Both InferXgate and LiteLLM are configured with identical resource constraints:

- **CPU**: 2 cores (1 core reserved)
- **Memory**: 1GB limit (512MB reserved)

## Services

The benchmark stack includes:

| Service | Port | Description |
|---------|------|-------------|
| InferXgate | 3000 | InferXgate gateway |
| LiteLLM | 4000 | LiteLLM proxy |
| Prometheus | 9090 | Metrics collection |
| cAdvisor | 8080 | Container metrics |
| InfluxDB | 8086 | k6 metrics storage |
| PostgreSQL | - | InferXgate database |
| Redis | - | InferXgate cache |

## Results

Results are saved to `k6/results/` with timestamps:

```
k6/results/
├── baseline-inferxgate-2024-01-15T10-30-00.json
├── baseline-litellm-2024-01-15T10-35-00.json
├── throughput-inferxgate-2024-01-15T10-40-00.json
├── throughput-litellm-2024-01-15T10-50-00.json
├── sustained-inferxgate-2024-01-15T11-00-00.json
└── sustained-litellm-2024-01-15T11-15-00.json
```

### Key Metrics

- **chat_completion_latency**: End-to-end request latency (ms)
- **time_to_first_byte**: Time until first response byte (ms)
- **throughput_total**: Total successful requests
- **tokens_per_second**: Token generation rate
- **error_rate**: Percentage of failed requests

## Monitoring

While tests are running:

- **Prometheus**: http://localhost:9090 - Query metrics
- **cAdvisor**: http://localhost:8080 - Container resource usage

## Troubleshooting

### Services not starting

Check Docker logs:

```bash
docker compose -f docker-compose.benchmark.yml logs inferxgate
docker compose -f docker-compose.benchmark.yml logs litellm
```

### API key errors

Ensure your `ANTHROPIC_API_KEY` is set correctly in `.env.benchmark` and is valid.

### High error rates

- Check if API rate limits are being hit
- Verify services are healthy: `curl http://localhost:3000/health`
- Review service logs for specific errors

### Clean up

```bash
# Stop services and remove volumes
docker compose -f docker-compose.benchmark.yml down -v
```
