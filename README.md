# Gen Serving Gateway

A unified AI model serving gateway for both **Image** and **Text** generation backends. Built with Rust for high performance and reliability.

[한국어 문서 (Korean)](README-KO.md)

## Features

### Core Capabilities
- **Unified API**: OpenAI-compatible API for both image and text generation
- **Multi-Backend Support**: Connect multiple AI backends (local or cloud)
- **Load Balancing**: Round Robin, Weighted, Least Connections strategies
- **Health Checking**: Automatic backend health monitoring with failover
- **Rate Limiting**: Configurable per-client and global rate limits
- **Authentication**: API key-based authentication

### Supported Backends

#### Image Generation
- Stable Diffusion (Automatic1111 WebUI, ComfyUI)
- DALL-E (OpenAI API)
- Midjourney API
- Custom HTTP/gRPC backends

#### Text Generation
- **OpenAI API** (GPT-4, GPT-3.5)
- **Anthropic** (Claude 3)
- **Ollama** (Local LLMs)
- **vLLM** (High-performance serving)
- **TGI** (Text Generation Inference)
- **Together AI**, **Groq**, and other OpenAI-compatible APIs

## Quick Start

### One-Line Installation (Ubuntu/Debian/macOS)

```bash
# Using Docker Compose (recommended)
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s compose

# Using Docker directly
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s docker
```

### Manual Installation

```bash
# Pull the Docker image
docker pull neuralfoundry2coder/gen-serving-gateway:latest

# Run with default configuration
docker run -d \
  --name gen-gateway \
  -p 15115:15115 \
  -v $(pwd)/config:/app/config:ro \
  neuralfoundry2coder/gen-serving-gateway:latest
```

## Configuration

### Backend Setup (Interactive)

Run the interactive setup script to configure your AI backends:

```bash
./scripts/setup-backends.sh
```

This will guide you through:
1. Adding image generation backends (SD, DALL-E, etc.)
2. Adding text generation backends (OpenAI, Ollama, etc.)
3. Configuring authentication and health checks
4. Testing backend connections

### Configuration Files

#### `config/backends.yaml` - AI Backend Configuration

```yaml
version: "1.0"

backends:
  # Image Generation Backends
  image:
    - name: stable-diffusion
      type: image
      protocol: http
      endpoints:
        - "http://localhost:8001"
      health_check:
        path: /internal/ping
        interval_secs: 30

  # Text Generation Backends  
  text:
    - name: ollama
      type: text
      protocol: openai
      endpoints:
        - "http://localhost:11434/v1"
      models:
        - llama3
        - mistral
      capabilities:
        - chat
        - completion

    - name: openai
      type: text
      protocol: openai
      endpoints:
        - "https://api.openai.com/v1"
      auth:
        type: bearer
        token_env: OPENAI_API_KEY
      models:
        - gpt-4
        - gpt-3.5-turbo
```

#### `config/gateway.yaml` - Gateway Configuration

```yaml
version: "1.0"

server:
  host: "0.0.0.0"
  port: 15115

auth:
  enabled: true
  api_keys:
    - "your-api-key-here"
  bypass_paths:
    - "/health"

rate_limit:
  enabled: true
  global:
    requests_per_second: 100
    burst_size: 200
  per_client:
    requests_per_second: 10
```

## API Reference

All endpoints are OpenAI-compatible.

### Image Generation

```bash
# Generate an image
curl -X POST http://localhost:15115/v1/images/generations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "prompt": "A beautiful sunset over mountains",
    "n": 1,
    "size": "1024x1024",
    "response_format": "url"
  }'
```

### Chat Completion

```bash
# Chat with an LLM
curl -X POST http://localhost:15115/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "llama3",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"}
    ]
  }'
```

### Text Completion

```bash
curl -X POST http://localhost:15115/v1/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "llama3",
    "prompt": "The quick brown fox",
    "max_tokens": 50
  }'
```

### List Models

```bash
curl http://localhost:15115/v1/models \
  -H "Authorization: Bearer your-api-key"
```

### Health Check

```bash
curl http://localhost:15115/health
```

### Backend Management

```bash
# List all backends
curl http://localhost:15115/v1/backends \
  -H "Authorization: Bearer your-api-key"

# List text backends
curl http://localhost:15115/v1/backends/text \
  -H "Authorization: Bearer your-api-key"

# Add a new backend
curl -X POST http://localhost:15115/v1/backends \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "name": "new-backend",
    "protocol": "http",
    "backend_type": "text",
    "endpoints": ["http://localhost:8000/v1"]
  }'

# Remove a backend
curl -X DELETE http://localhost:15115/v1/backends/backend-name \
  -H "Authorization: Bearer your-api-key"
```

## Docker Hub

The official Docker image is available on Docker Hub:

```bash
docker pull neuralfoundry2coder/gen-serving-gateway:latest
```

### Available Tags

- `latest` - Latest stable release
- `x.y.z` - Specific version (e.g., `0.3.0`)
- `sha-xxxxxx` - Specific commit

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/neuralfoundry-coder/gen-serving-gateway.git
cd gen-serving-gateway

# Build
cargo build --release

# Run
./target/release/gen-gateway
```

### Running Tests

```bash
# Unit tests
cargo test

# With mock backends (requires Docker)
./docker/scripts/start-test-env.sh
cargo test --features integration
./docker/scripts/stop-test-env.sh
```

### Release & Deployment

For maintainers:

```bash
# Interactive release (prompts for version bump)
./scripts/deploy.sh release

# Direct Docker Hub push
./scripts/deploy.sh direct

# Release with specific version
./scripts/deploy.sh release -v 1.0.0
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Gen Serving Gateway                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐│
│  │   Auth      │ │ Rate Limit  │ │       Router            ││
│  │  Middleware │ │  Middleware │ │  /v1/images/generations ││
│  └─────────────┘ └─────────────┘ │  /v1/chat/completions   ││
│                                   │  /v1/completions        ││
│                                   │  /v1/models             ││
│                                   └─────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌─────────────────────────────────┤
│  │   Load Balancer     │  │      Request Queue              │
│  │  (RR, Weighted, LC) │  │      (Async, Batching)          │
│  └─────────────────────┘  └─────────────────────────────────┤
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Backend Registry                          │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │  │
│  │  │ Image   │ │  Text   │ │  gRPC   │ │ Health  │     │  │
│  │  │ Backend │ │ Backend │ │ Backend │ │ Checker │     │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘     │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │ Stable   │   │  Ollama  │   │  OpenAI  │
        │ Diffusion│   │  (Local) │   │   API    │
        └──────────┘   └──────────┘   └──────────┘
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `GEN_GATEWAY__SERVER__HOST` | Server bind address | `0.0.0.0` |
| `GEN_GATEWAY__SERVER__PORT` | Server port | `15115` |
| `GEN_GATEWAY__AUTH__ENABLED` | Enable authentication | `true` |
| `RUST_LOG` | Log level | `info` |
| `OPENAI_API_KEY` | OpenAI API key (for OpenAI backend) | - |
| `ANTHROPIC_API_KEY` | Anthropic API key (for Claude backend) | - |

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
