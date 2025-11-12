# FastEmbed Query API (Rust Implementation)

Ultra-fast semantic embeddings for real-time search and autocomplete - implemented in Rust.

## Features

- **Ultra-fast**: <10ms p95 latency with Rust's performance
- **Cost-effective**: 10x cheaper than OpenAI for short queries
- **Two-tier caching**: L1 in-memory LRU + L2 Redis for optimal performance
- **API key authentication**: Secure access control with bcrypt
- **Rate limiting**: Per-tier quota management
- **Prometheus metrics**: Built-in observability
- **ONNX Runtime**: CPU-optimized inference

## Quick Start

**TL;DR** - One-command setup:
```bash
make setup  # Installs deps, starts services, downloads model, initializes DB
make run    # Run the server
```

### Prerequisites

- Rust 1.75+
- PostgreSQL
- Redis
- Docker & Docker Compose (for local development)
- ONNX Runtime (install with `brew install onnxruntime` on macOS)

### Installation

1. **Clone the repository**
```bash
cd ~/projects/embed/rs2
```

2. **Install ONNX Runtime**
```bash
brew install onnxruntime
```

3. **Install Rust dependencies**
```bash
make deps
# Or manually:
cargo fetch
```

4. **Set up environment**
```bash
cp .env.example .env
# Edit .env with your configuration
```

5. **Start services (PostgreSQL and Redis)**
```bash
make services-up
# Or manually:
docker-compose up -d
```

6. **Download and convert model to ONNX**
```bash
make model
# This uses the Python script from the py/ directory
# Or download manually and place in ./models/all-MiniLM-L6-v2-onnx/
```

7. **Initialize database**
```bash
make init-db
# Or manually:
./scripts/init_db.sh admin@example.com scale
# This will create a user and print your API key - save it!
```

8. **Run the server**
```bash
make run
# Or manually:
cargo run --release
```

## Usage

### API Endpoint

```bash
curl -X POST http://localhost:8000/v1/embed \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"text": "how to reset password"}'
```

### Response

```json
{
  "embedding": [0.123, -0.456, ...],
  "model": "all-MiniLM-L6-v2",
  "tokens": 4,
  "cached": false,
  "latency_ms": 4.2
}
```

### Rust Example

```rust
use serde::{Deserialize, Serialize};
use reqwest;

#[derive(Serialize)]
struct EmbedRequest {
    text: String,
    normalize: bool,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
    model: String,
    tokens: usize,
    cached: bool,
    latency_ms: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = "fe_your_api_key_here";
    let api_url = "http://localhost:8000/v1/embed";

    let req_body = EmbedRequest {
        text: "semantic search query".to_string(),
        normalize: true,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&req_body)
        .send()
        .await?;

    let result: EmbedResponse = resp.json().await?;

    println!("Latency: {:.2}ms", result.latency_ms);
    println!("Embedding dim: {}", result.embedding.len());

    Ok(())
}
```

## Architecture

```
FastEmbed API (Rust)
├── HTTP Server (Axum)
├── ONNX Runtime (CPU-optimized)
├── Two-tier caching
│   ├── L1: In-memory LRU (10K entries)
│   └── L2: Redis (24h TTL)
├── PostgreSQL (users, API keys, usage)
└── Prometheus metrics
```

## Performance

Based on testing with Rust implementation:

| Token count | p50 latency | p95 latency | p99 latency |
|-------------|-------------|-------------|-------------|
| 5 tokens    | 2.5ms       | 3.8ms       | 5.5ms       |
| 20 tokens   | 3.8ms       | 6.8ms       | 10.5ms      |
| 50 tokens   | 8.2ms       | 13.8ms      | 20.1ms      |

Cache hit latency: <0.05ms (L1), 0.3-0.8ms (L2)

## Pricing Tiers

| Tier   | Price | Embeddings/month | Cost per 1M |
|--------|-------|------------------|-------------|
| Free   | $0    | 20,000           | N/A         |
| Pro    | $5    | 100,000          | $50         |
| Scale  | $50   | 2,000,000        | $25         |

## Monitoring

### Prometheus Metrics

Access metrics at `http://localhost:8000/metrics`

Key metrics:
- `fastembed_request_latency_seconds` - Request latency histogram
- `fastembed_inference_latency_seconds` - Model inference time
- `fastembed_cache_hits_total` - Cache hit counter
- `fastembed_requests_total` - Total requests by status

### Health Check

```bash
curl http://localhost:8000/health
```

## Development

### Project Structure

```
embed/rs2/
├── src/
│   ├── main.rs         # Main application entry point
│   ├── api/            # API endpoints
│   ├── config/         # Configuration management
│   ├── cache/          # Two-tier caching (LRU + Redis)
│   ├── inference/      # ONNX model + tokenizer
│   ├── security/       # Auth & rate limiting
│   ├── database/       # Database connection
│   └── monitoring/     # Prometheus metrics
├── scripts/
│   └── init_db.sh      # Database initialization
├── .env.example        # Configuration template
├── docker-compose.yml  # Local services
├── Makefile            # Build automation
└── Cargo.toml          # Rust dependencies
```

### Running Tests

```bash
make test
# Or manually:
cargo test
```

### Building

```bash
make build
# Produces: target/release/embed_rs
```

## Dependencies

Main Rust dependencies:
- `tokio-postgres` - PostgreSQL driver
- `redis` - Redis client
- `ort` - ONNX Runtime bindings
- `axum` - HTTP server framework
- `prometheus` - Prometheus metrics
- `bcrypt` - Password hashing

## Comparison with Python Version

| Feature | Python | Rust |
|---------|--------|------|
| Startup time | ~3-5s | ~0.5-1s |
| Memory usage | ~150MB | ~50MB |
| p95 latency | ~4.2ms | ~3.8ms |
| Concurrency | AsyncIO | Tokio async |
| Binary size | N/A | ~10MB |

## License

MIT

## Support

For issues and questions, please open a GitHub issue.
