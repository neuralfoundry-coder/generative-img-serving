# Generative Image Serving Framework

**English** | [한국어](README-KO.md)

A Rust-based unified serving framework for generative image models. Integrate and manage multiple image generation backends through a single gateway.

## Features

- **Multi-Backend Integration**: HTTP/gRPC protocol support, connect various image generation model backends
- **Gateway Functions**: Load balancing, dynamic routing, API authentication, rate limiting
- **Async Processing**: Async request queue, dynamic batch processing
- **Fault Tolerance**: Health checks, automatic failover, circuit breaker pattern
- **Flexible Response Formats**: Base64 encoding, file storage, URL references
- **OpenAI API Compatible**: Compatibility with existing clients

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                               │
│              (HTTP Client, SDK, etc.)                        │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                    Gateway Layer                             │
│  ┌──────────┐  ┌────────────┐  ┌──────────┐  ┌───────────┐ │
│  │ Axum HTTP│  │ API Key    │  │ Rate     │  │ Dynamic   │ │
│  │ Server   │→ │ Auth       │→ │ Limiter  │→ │ Router    │ │
│  └──────────┘  └────────────┘  └──────────┘  └───────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                     Core Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ Request Queue│→ │ Batcher      │→ │ Load Balancer     │  │
│  └──────────────┘  └──────────────┘  └─────────┬─────────┘  │
│                                                 │            │
│  ┌──────────────────────────────────────────────▼─────────┐ │
│  │              Health Check Manager                       │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   Backend Pool                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ SD Backend   │  │ DALL-E       │  │ Custom Model     │   │
│  │ (HTTP)       │  │ (HTTP)       │  │ (gRPC)           │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### One-Line Installation (Recommended)

Deploy instantly with a single command:

```bash
# Docker Compose (Recommended)
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s compose

# Docker Direct
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s docker
```

**Supported Operating Systems:**

| OS | Package Manager | Status |
|----|-----------------|--------|
| Ubuntu / Debian | apt | ✅ |
| CentOS / RHEL / Rocky / AlmaLinux | yum | ✅ |
| Fedora | dnf | ✅ |
| Amazon Linux | yum | ✅ |
| macOS | Homebrew | ✅ |

**Installation Options:**

```bash
# Custom port
curl -fsSL .../quick-install.sh | HOST_PORT=9090 bash -s compose

# Custom install directory
curl -fsSL .../quick-install.sh | INSTALL_DIR=/opt/gen-gateway bash -s compose

# Specific version
curl -fsSL .../quick-install.sh | IMAGE_TAG=0.2.0 bash -s compose
```

### Build from Source

#### Requirements

- Rust 1.83+
- (Optional) protoc for gRPC features

#### Installation

```bash
# Clone repository
git clone https://github.com/neuralfoundry-coder/gen-serving-gateway.git
cd gen-serving-gateway

# Build
cargo build --release

# Run
./target/release/gen-gateway
```

### Configuration

Edit `config/default.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
enabled = true
api_keys = ["your-api-key"]

[rate_limit]
enabled = true
requests_per_second = 100
burst_size = 200

[[backends]]
name = "stable-diffusion"
protocol = "http"
endpoints = ["http://localhost:7860"]
health_check_path = "/health"
health_check_interval_secs = 30
timeout_ms = 60000
weight = 1
enabled = true
```

Environment variables (prefix: `IMG_SERVING__`):

```bash
export IMG_SERVING__SERVER__PORT=9090
export IMG_SERVING__AUTH__ENABLED=false
```

## API Endpoints

### Image Generation (OpenAI Compatible)

```bash
curl -X POST http://localhost:8080/v1/images/generations \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "A beautiful sunset over mountains",
    "n": 1,
    "size": "1024x1024",
    "response_format": "url"
  }'
```

**Response:**
```json
{
  "created": 1234567890,
  "data": [
    {
      "url": "http://localhost:8080/images/abc123.png"
    }
  ]
}
```

### Extended Parameters

Additional parameters beyond OpenAI API:

```json
{
  "prompt": "...",
  "negative_prompt": "blurry, low quality",
  "seed": 42,
  "guidance_scale": 7.5,
  "num_inference_steps": 50,
  "backend": "stable-diffusion"
}
```

### Backend Management

```bash
# List backends
curl http://localhost:8080/v1/backends \
  -H "Authorization: Bearer your-api-key"

# Add backend
curl -X POST http://localhost:8080/v1/backends \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "new-backend",
    "protocol": "http",
    "endpoints": ["http://gpu-server:7860"]
  }'

# Remove backend
curl -X DELETE http://localhost:8080/v1/backends/backend-name \
  -H "Authorization: Bearer your-api-key"
```

### Health Check

```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "0.2.0",
  "backends": {
    "total": 2,
    "healthy": 2,
    "unhealthy": 0
  }
}
```

## Load Balancing Strategies

- **Round Robin** (default): Distribute requests sequentially
- **Weighted Round Robin**: Weight-based distribution
- **Random**: Random selection
- **Least Connections**: Connection count based (coming soon)

## Response Formats

| Format | Description |
|--------|-------------|
| `url` | Return URL of generated image (default) |
| `b64_json` | Return Base64 encoded image data |
| `file` | Return local file path (internal use) |

## Docker Deployment

### Docker Hub Image

```bash
# Latest version
docker pull neuralfoundry2coder/gen-serving-gateway:latest

# Specific version
docker pull neuralfoundry2coder/gen-serving-gateway:0.2.0
```

### Manual Docker Run

```bash
docker run -d \
  --name gen-gateway \
  -p 8080:8080 \
  -v $(pwd)/config:/app/config \
  -v $(pwd)/generated_images:/app/generated_images \
  -e RUST_LOG=info \
  --restart unless-stopped \
  neuralfoundry2coder/gen-serving-gateway:latest
```

### Docker Compose

```yaml
# docker-compose.yml
services:
  gen-gateway:
    image: neuralfoundry2coder/gen-serving-gateway:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config:/app/config:ro
      - ./generated_images:/app/generated_images
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

```bash
docker compose up -d
```

## Development

```bash
# Development mode
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint check
cargo clippy
```

## gRPC Support

Install `protoc` for gRPC backends:

```bash
# macOS
brew install protobuf

# Ubuntu
apt-get install protobuf-compiler

# Build with gRPC codegen
cargo build --features grpc-codegen
```

## Project Structure

```
src/
├── main.rs                 # Entry point
├── lib.rs                  # Library root
├── config/                 # Configuration
├── api/                    # HTTP API
│   ├── routes.rs          # Route definitions
│   ├── handlers.rs        # Request handlers
│   └── models.rs          # API models
├── middleware/            # Middleware
│   ├── auth.rs            # API Key authentication
│   └── rate_limit.rs      # Rate limiting
├── gateway/               # Gateway
│   ├── load_balancer.rs   # Load balancer
│   ├── health_check.rs    # Health checks
│   └── router.rs          # Dynamic router
├── queue/                 # Request processing
│   ├── request_queue.rs   # Async queue
│   └── batcher.rs         # Batch processing
├── backend/               # Backend integration
│   ├── traits.rs          # Common traits
│   ├── http_backend.rs    # HTTP client
│   ├── grpc_backend.rs    # gRPC client
│   └── registry.rs        # Backend registry
├── response/              # Response handling
│   ├── base64.rs          # Base64 encoding
│   ├── file.rs            # File storage
│   └── url.rs             # URL generation
└── error.rs               # Error handling

deploy/
├── quick-install.sh       # One-line installer (multi-OS)
├── deploy-docker.sh       # Docker direct deployment
├── deploy-compose.sh      # Docker Compose deployment
└── docker-compose.yml     # Compose configuration

scripts/
├── deploy.sh              # Release & deployment script
├── test-runner.sh         # Test runner
└── ci-test.sh             # CI/CD test script
```

---

## Release & Deployment (For Developers)

### Deployment Script (`scripts/deploy.sh`)

Unified deployment script for Docker Hub publishing.

#### 1. Direct Push Mode

```bash
# Setup .env file (first time only)
cp .env.example .env

# Build and push
./scripts/deploy.sh direct

# Push specific version
./scripts/deploy.sh direct -v 1.0.0
```

#### 2. Release Mode (via GitHub Actions)

```bash
# Interactive version selection + auto commit/push/tag
./scripts/deploy.sh release

# Version selection prompt:
#   [1] Major  : v0.2.0 → v1.0.0  (Breaking changes)
#   [2] Minor  : v0.2.0 → v0.3.0  (New features)
#   [3] Patch  : v0.2.0 → v0.2.1  (Bug fixes)
#   [4] Custom : Enter custom version

# Release specific version (skip prompt)
./scripts/deploy.sh release -v 1.0.0

# Dry run
./scripts/deploy.sh release -d
```

### GitHub Actions Setup

Configure repository secrets for automated deployment:

1. Go to GitHub repository Settings → Secrets and variables → Actions
2. Add the following secrets:
   - `DOCKER_USERNAME`: Docker Hub username
   - `DOCKER_ACCESS_TOKEN`: Docker Hub access token

**Auto Triggers:**
- `main` branch push → Auto build and push
- Tag creation (`v*`) → Push with version tag
- Pull Request → Build test only (no push)
- Manual → workflow_dispatch in Actions tab

## License

MIT License
