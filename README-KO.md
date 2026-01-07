# Gen Serving Gateway

**이미지**와 **텍스트** 생성 백엔드를 위한 통합 AI 모델 서빙 게이트웨이입니다. 고성능과 안정성을 위해 Rust로 개발되었습니다.

[English Documentation](README.md)

## 특징

### 핵심 기능
- **통합 API**: 이미지 및 텍스트 생성을 위한 OpenAI 호환 API
- **다중 백엔드 지원**: 로컬 또는 클라우드의 여러 AI 백엔드 연결
- **로드 밸런싱**: Round Robin, 가중치 기반, Least Connections 전략
- **헬스 체크**: 자동 백엔드 상태 모니터링 및 장애 조치
- **속도 제한**: 클라이언트별 및 전역 속도 제한 설정
- **인증**: API 키 기반 인증

### 지원 백엔드

#### 이미지 생성
- Stable Diffusion (Automatic1111 WebUI, ComfyUI)
- DALL-E (OpenAI API)
- Midjourney API
- 커스텀 HTTP/gRPC 백엔드

#### 텍스트 생성
- **OpenAI API** (GPT-4, GPT-3.5)
- **Anthropic** (Claude 3)
- **Ollama** (로컬 LLM)
- **vLLM** (고성능 서빙)
- **TGI** (Text Generation Inference)
- **Together AI**, **Groq** 및 기타 OpenAI 호환 API

## 빠른 시작

### 원라인 설치 (Ubuntu/Debian/macOS)

```bash
# Docker Compose 사용 (권장)
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s compose

# Docker 직접 사용
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s docker
```

### 수동 설치

```bash
# Docker 이미지 풀
docker pull neuralfoundry2coder/gen-serving-gateway:latest

# 기본 설정으로 실행
docker run -d \
  --name gen-gateway \
  -p 8080:8080 \
  -v $(pwd)/config:/app/config:ro \
  neuralfoundry2coder/gen-serving-gateway:latest
```

## 설정

### 백엔드 설정 (대화식)

AI 백엔드를 구성하려면 대화식 설정 스크립트를 실행하세요:

```bash
./scripts/setup-backends.sh
```

다음 항목을 안내합니다:
1. 이미지 생성 백엔드 추가 (SD, DALL-E 등)
2. 텍스트 생성 백엔드 추가 (OpenAI, Ollama 등)
3. 인증 및 헬스 체크 구성
4. 백엔드 연결 테스트

### 설정 파일

#### `config/backends.yaml` - AI 백엔드 설정

```yaml
version: "1.0"

backends:
  # 이미지 생성 백엔드
  image:
    - name: stable-diffusion
      type: image
      protocol: http
      endpoints:
        - "http://localhost:7860"
      health_check:
        path: /internal/ping
        interval_secs: 30

  # 텍스트 생성 백엔드
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

#### `config/gateway.yaml` - 게이트웨이 설정

```yaml
version: "1.0"

server:
  host: "0.0.0.0"
  port: 8080

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

## API 참조

모든 엔드포인트는 OpenAI 호환입니다.

### 이미지 생성

```bash
# 이미지 생성
curl -X POST http://localhost:8080/v1/images/generations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "prompt": "산 위의 아름다운 일몰",
    "n": 1,
    "size": "1024x1024",
    "response_format": "url"
  }'
```

### 채팅 완성

```bash
# LLM과 채팅
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "llama3",
    "messages": [
      {"role": "user", "content": "안녕하세요, 오늘 기분이 어떠신가요?"}
    ]
  }'
```

### 텍스트 완성

```bash
curl -X POST http://localhost:8080/v1/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "llama3",
    "prompt": "빠른 갈색 여우가",
    "max_tokens": 50
  }'
```

### 모델 목록

```bash
curl http://localhost:8080/v1/models \
  -H "Authorization: Bearer your-api-key"
```

### 헬스 체크

```bash
curl http://localhost:8080/health
```

### 백엔드 관리

```bash
# 모든 백엔드 목록
curl http://localhost:8080/v1/backends \
  -H "Authorization: Bearer your-api-key"

# 텍스트 백엔드 목록
curl http://localhost:8080/v1/backends/text \
  -H "Authorization: Bearer your-api-key"

# 새 백엔드 추가
curl -X POST http://localhost:8080/v1/backends \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "name": "new-backend",
    "protocol": "http",
    "backend_type": "text",
    "endpoints": ["http://localhost:8000/v1"]
  }'

# 백엔드 제거
curl -X DELETE http://localhost:8080/v1/backends/backend-name \
  -H "Authorization: Bearer your-api-key"
```

## Docker Hub

공식 Docker 이미지는 Docker Hub에서 제공됩니다:

```bash
docker pull neuralfoundry2coder/gen-serving-gateway:latest
```

### 사용 가능한 태그

- `latest` - 최신 안정 릴리스
- `x.y.z` - 특정 버전 (예: `0.3.0`)
- `sha-xxxxxx` - 특정 커밋

## 개발

### 소스에서 빌드

```bash
# 저장소 복제
git clone https://github.com/neuralfoundry-coder/gen-serving-gateway.git
cd gen-serving-gateway

# 빌드
cargo build --release

# 실행
./target/release/gen-gateway
```

### 테스트 실행

```bash
# 단위 테스트
cargo test

# 모의 백엔드 사용 (Docker 필요)
./docker/scripts/start-test-env.sh
cargo test --features integration
./docker/scripts/stop-test-env.sh
```

### 릴리스 및 배포

메인테이너용:

```bash
# 대화식 릴리스 (버전 범프 프롬프트)
./scripts/deploy.sh release

# Docker Hub 직접 푸시
./scripts/deploy.sh direct

# 특정 버전으로 릴리스
./scripts/deploy.sh release -v 1.0.0
```

## 아키텍처

```
┌─────────────────────────────────────────────────────────────┐
│                    Gen Serving Gateway                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐│
│  │   인증      │ │  속도 제한  │ │       라우터            ││
│  │  미들웨어   │ │  미들웨어   │ │  /v1/images/generations ││
│  └─────────────┘ └─────────────┘ │  /v1/chat/completions   ││
│                                   │  /v1/completions        ││
│                                   │  /v1/models             ││
│                                   └─────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌─────────────────────────────────┤
│  │   로드 밸런서       │  │      요청 큐                    │
│  │  (RR, 가중치, LC)   │  │      (비동기, 배칭)             │
│  └─────────────────────┘  └─────────────────────────────────┤
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────┐  │
│  │              백엔드 레지스트리                          │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │  │
│  │  │ 이미지  │ │  텍스트 │ │  gRPC   │ │ 헬스    │     │  │
│  │  │ 백엔드  │ │ 백엔드  │ │ 백엔드  │ │ 체커    │     │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘     │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │ Stable   │   │  Ollama  │   │  OpenAI  │
        │ Diffusion│   │  (로컬)  │   │   API    │
        └──────────┘   └──────────┘   └──────────┘
```

## 환경 변수

| 변수 | 설명 | 기본값 |
|------|------|--------|
| `GEN_GATEWAY__SERVER__HOST` | 서버 바인드 주소 | `0.0.0.0` |
| `GEN_GATEWAY__SERVER__PORT` | 서버 포트 | `8080` |
| `GEN_GATEWAY__AUTH__ENABLED` | 인증 활성화 | `true` |
| `RUST_LOG` | 로그 레벨 | `info` |
| `OPENAI_API_KEY` | OpenAI API 키 (OpenAI 백엔드용) | - |
| `ANTHROPIC_API_KEY` | Anthropic API 키 (Claude 백엔드용) | - |

## 라이선스

MIT 라이선스 - 자세한 내용은 [LICENSE](LICENSE)를 참조하세요.

## 기여

기여를 환영합니다! Pull Request를 자유롭게 제출해 주세요.

1. 저장소 포크
2. 기능 브랜치 생성 (`git checkout -b feature/amazing-feature`)
3. 변경 사항 커밋 (`git commit -m 'Add amazing feature'`)
4. 브랜치에 푸시 (`git push origin feature/amazing-feature`)
5. Pull Request 열기
