# InferXgate - Blazing fast LLM Gateway for 100+ LLMs

A high-performance LLM Gateway built in Rust with a React dashboard that provides a unified OpenAI-compatible API interface for multiple LLM providers.

![InferXgate Dashboard](docs/images/dashboard.png)

## Features

### Currently Supported Providers
- ✅ **Anthropic** (Claude 4.5, 4.1, 4.0, 3.5, 3)
- ✅ **Google Gemini** (Gemini 2.5 Pro, Flash, 2.0 Flash)
- ✅ **OpenAI** (GPT-5, GPT-4.1, GPT-4 Turbo)
- ✅ **Azure OpenAI** (All Azure-deployed OpenAI models)

### Coming Soon
- 🚧 AWS Bedrock
- 🚧 Google VertexAI
- 🚧 Cohere
- 🚧 HuggingFace
- 🚧 Replicate
- 🚧 Groq

### Core Features
- 🚀 **OpenAI-Compatible API**: Drop-in replacement for OpenAI API
- ⚡ **High Performance**: Built with Rust and Tokio for maximum throughput
- 🔄 **Streaming Support**: Server-Sent Events for real-time responses
- 🎯 **Smart Routing**: Automatic model-to-provider mapping
- 📊 **Analytics Dashboard**: Real-time metrics and monitoring
- 🔒 **Security**: API key management, rate limiting, IP whitelisting
- 💾 **Caching**: Response caching with Redis (optional)
- 🔁 **Retry Logic**: Automatic retries with exponential backoff
- 📝 **Comprehensive Logging**: Structured logging with tracing

## Quick Start

### Quick Production Setup (Recommended)

The fastest way to get started using pre-built Docker images:

```bash
# Clone the repository
git clone https://github.com/jasmedia/InferXgate.git
cd InferXgate

# Set up environment variables
cp .env.example .env
# Edit .env with your API keys (at minimum, set one LLM provider key)

# Run the quickstart script and select option 4
./quickstart.sh
# Choose: 4) Run with Docker (production - uses pre-built images)
```

Or manually:
```bash
docker-compose -f docker-compose.prod.yml up -d
```

Services will be available at:
- Frontend: http://localhost
- Backend API: http://localhost:3000

### Development Setup

For development, you'll need:
- Rust 1.75+ (install from [rustup.rs](https://rustup.rs))
- Bun 1.0+ (install from [bun.sh](https://bun.sh))
- API keys for providers you want to use

### Backend Setup (Development)

1. Clone the repository:
```bash
git clone https://github.com/jasmedia/InferXgate.git
cd InferXgate
```

2. Set up environment variables:
```bash
cd backend
cp .env.example .env
# Edit .env with your API keys
```

3. Build and run the Rust backend:
```bash
cargo build --release
cargo run --release
```

The backend will start on `http://localhost:3000`

### Frontend Setup

1. Install dependencies:
```bash
cd frontend
bun install
```

2. Start the development server:
```bash
bun run dev
```

The frontend will start on `http://localhost:5173`

## Configuration

### Environment Variables

Create a `.env` file in the backend directory:

```env
# Server Configuration
HOST=0.0.0.0
PORT=3000
LOG_LEVEL=info

# Provider API Keys
ANTHROPIC_API_KEY=your-anthropic-api-key
GEMINI_API_KEY=your-gemini-api-key
OPENAI_API_KEY=your-openai-api-key

# Azure OpenAI Configuration
AZURE_OPENAI_API_KEY=your-azure-openai-api-key
AZURE_OPENAI_ENDPOINT=https://your-resource.openai.azure.com
AZURE_OPENAI_API_VERSION=2024-02-15-preview
AZURE_OPENAI_RESOURCE_NAME=your-resource-name

COHERE_API_KEY=your-cohere-api-key

# AWS Configuration (for Bedrock)
AWS_ACCESS_KEY_ID=your-aws-access-key
AWS_SECRET_ACCESS_KEY=your-aws-secret-key
AWS_REGION=us-east-1

# Optional: Database for metadata
DATABASE_URL=postgresql://user:password@localhost/inferxgate

# Optional: Redis for caching
REDIS_URL=redis://localhost:6379
```

## API Usage

The gateway provides an OpenAI-compatible API. You can use it with any OpenAI client library.

### Using with OpenAI Python SDK

```python
from openai import OpenAI

# Point to your gateway instead of OpenAI
client = OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="your-gateway-api-key"  # If auth is enabled
)

# Use any supported model
response = client.chat.completions.create(
    model="claude-3-opus-20240229",  # or "gemini-1.5-pro"
    messages=[
        {"role": "user", "content": "Hello, how are you?"}
    ],
    temperature=0.7,
    max_tokens=1000
)

print(response.choices[0].message.content)
```

### Using with curl

```bash
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-opus-20240229",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "temperature": 0.7,
    "max_tokens": 1000
  }'
```

### Streaming Responses

```python
stream = client.chat.completions.create(
    model="gemini-1.5-pro",
    messages=[{"role": "user", "content": "Write a story"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

## API Endpoints

### Core Endpoints

| Endpoint | Method | Description |
|----------|---------|-------------|
| `/v1/chat/completions` | POST | Create chat completion |
| `/v1/models` | GET | List available models |
| `/health` | GET | Health check |

### Request Format

```json
{
  "model": "claude-3-opus-20240229",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "top_p": 0.9,
  "frequency_penalty": 0,
  "presence_penalty": 0,
  "stop": ["\\n\\n"],
  "stream": false,
  "n": 1,
  "user": "user-123"
}
```

### Response Format

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1699000000,
  "model": "claude-3-opus-20240229",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well, thank you. How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 15,
    "total_tokens": 25
  }
}
```

## Development

### Project Structure

```
llm-gateway/
├── backend/
│   ├── src/
│   │   ├── main.rs           # Main server entry point
│   │   ├── config.rs         # Configuration management
│   │   ├── error.rs          # Error handling
│   │   └── providers/        # Provider implementations
│   │       ├── mod.rs        # Provider trait
│   │       ├── anthropic.rs  # Anthropic provider
│   │       ├── gemini.rs     # Gemini provider
│   │       ├── openai.rs     # OpenAI provider
│   │       └── azure.rs      # Azure OpenAI provider
│   └── Cargo.toml
├── frontend/
│   ├── src/
│   │   ├── App.tsx           # Main React app
│   │   ├── components/       # React components
│   │   └── pages/            # Page components
│   ├── package.json
│   └── vite.config.ts
└── docker-compose.yml
```

### Adding a New Provider

1. Create a new file in `backend/src/providers/`:
```rust
// backend/src/providers/openai.rs
use async_trait::async_trait;
use crate::providers::LLMProvider;

pub struct OpenAIProvider {
    client: reqwest::Client,
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    // Implement required methods
}
```

2. Register the provider in `main.rs`:
```rust
providers.insert("openai".to_string(), Box::new(OpenAIProvider::new()));
```

3. Add model mappings:
```rust
model_routes.insert(
    "gpt-4".to_string(),
    ModelRoute {
        provider: "openai".to_string(),
        target_model: "gpt-4".to_string(),
        api_key: api_key.clone(),
    },
);
```

## Docker Deployment

### Production Deployment (Recommended)

Use pre-built images from Docker Hub:

```bash
# Set up environment variables
cp .env.example .env
# Edit .env with your API keys and secrets

# Start with production compose file
docker-compose -f docker-compose.prod.yml up -d
```

This uses:
- `inferxgate/backend:latest`
- `inferxgate/frontend:latest`

### Development with Docker Compose

Build images locally:

```bash
# Copy the example env file
cp .env.example .env

# Edit .env with your API keys
# At minimum, set one LLM provider API key (e.g., ANTHROPIC_API_KEY)

# Start all services
docker-compose up -d
```

### Building Images Separately

Backend:
```bash
cd backend
docker build -t llm-gateway-backend .
docker run -p 3000:3000 --env-file .env llm-gateway-backend
```

Frontend:
```bash
cd frontend
docker build -t llm-gateway-frontend .
docker run -p 80:80 llm-gateway-frontend
```

## Available Make Commands

The project includes a comprehensive Makefile for common tasks (all frontend commands use Bun):

```bash
make help           # Show all available commands
make setup          # Initial setup (copy .env.example, install deps)
make install        # Install all dependencies (Rust + Bun)
make dev            # Start development servers (backend + frontend)
make build          # Build both backend and frontend for production
make test           # Run all tests (backend + frontend)
make fmt            # Format code (cargo fmt + bun run format)
make lint           # Lint code (cargo clippy + bun run lint)
make docker-build   # Build Docker images
make docker-up      # Start all services with Docker Compose
make docker-down    # Stop Docker services
```

## Performance Optimization

### Recommended Settings

1. **Connection Pooling**: The gateway uses connection pooling by default
2. **Response Caching**: Enable Redis for caching frequent requests
3. **Rate Limiting**: Configure appropriate rate limits per provider
4. **Timeouts**: Adjust timeouts based on model response times

### Benchmarks

- Latency overhead: < 5ms
- Throughput: 10,000+ requests/second
- Memory usage: ~50MB base + cache

## Monitoring

The gateway provides several monitoring endpoints:

- `/metrics` - Prometheus metrics
- `/health` - Health check
- Dashboard at `http://localhost:5173` - Real-time analytics

## Troubleshooting

### Common Issues

1. **Rate limit errors**: Check provider rate limits and adjust retry settings
2. **Timeout errors**: Increase timeout values for larger models
3. **Authentication errors**: Verify API keys in `.env` file
4. **CORS errors**: Ensure frontend proxy is configured correctly

### Debug Mode

Enable debug logging:
```bash
LOG_LEVEL=debug cargo run
```

## Security Best Practices

1. **Use HTTPS in production**: Deploy behind a reverse proxy with SSL
2. **Rotate API keys regularly**: Set up key rotation policies
3. **Enable authentication**: Use the built-in auth middleware
4. **IP whitelisting**: Restrict access to known IPs
5. **Rate limiting**: Prevent abuse with appropriate limits

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Roadmap

- [x] Core gateway functionality
- [x] Anthropic provider
- [x] Google Gemini provider
- [x] OpenAI provider
- [x] Azure OpenAI provider
- [x] React dashboard
- [x] Streaming support
- [ ] AWS Bedrock provider
- [ ] Redis caching
- [ ] PostgreSQL for metadata
- [ ] Prometheus metrics
- [ ] Advanced routing strategies
- [ ] Load balancing
- [ ] A/B testing support
- [ ] Cost tracking
- [ ] Semantic caching
- [ ] Request queuing
- [ ] WebSocket support

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [LiteLLM](https://github.com/BerriAI/litellm)
- Built with [Axum](https://github.com/tokio-rs/axum) web framework
- UI components from [Tailwind CSS](https://tailwindcss.com)

## Support

For issues and questions:
- Email: support@inferxgate.com
- Open an issue on [GitHub](https://github.com/jasmedia/InferXgate/issues)

---

**Note**: This is an active development project. APIs may change between versions.
