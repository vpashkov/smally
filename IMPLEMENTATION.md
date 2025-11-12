# Rust Implementation Details

This document describes the Rust implementation of the FastEmbed API, which is a port of the Python version.

## Architecture Overview

The Rust implementation follows the same architecture as the Python version but leverages Rust's performance and safety characteristics:

```
src/
├── main.rs          - Main application entry point
├── api/             - HTTP handlers for API endpoints (Axum)
├── config/          - Configuration management with env vars
├── cache/           - Two-tier caching (L1 LRU + L2 Redis)
├── inference/       - ONNX Runtime integration + custom tokenizer
├── security/        - API key auth (bcrypt) + rate limiting
├── database/        - PostgreSQL connection pooling (tokio-postgres)
└── monitoring/      - Prometheus metrics
```

## Key Implementation Choices

### 1. ONNX Runtime Integration

- Uses `ort` crate for BERT model inference
- Custom WordPiece tokenizer implementation in pure Rust
- Mean pooling and L2 normalization implemented from scratch
- Zero-copy operations where possible for performance

### 2. Concurrency Model

- Uses Tokio async runtime instead of Python's AsyncIO
- Connection pooling via tokio-postgres for PostgreSQL
- LRU cache with Arc<RwLock> for thread-safe concurrent access
- Lock-free operations optimized with async/await

### 3. HTTP Server

- Axum framework instead of FastAPI
- Tower middleware for CORS, logging, and timing
- Graceful shutdown with signal handling

### 4. Caching Strategy

**L1 Cache (In-Memory):**
- Custom LRU implementation with HashMap and doubly-linked list
- Thread-safe with Arc<RwLock<T>>
- O(1) get/set operations

**L2 Cache (Redis):**
- Async writes using Tokio tasks
- Binary serialization (Vec<f32> -> bytes)
- 24-hour TTL

### 5. Database Access

- `tokio-postgres` for optimal PostgreSQL performance
- Connection pooling with deadpool
- Parameterized queries for security

### 6. Security

- bcrypt for API key hashing (same as Python)
- Per-request rate limiting with PostgreSQL-backed usage tracking
- Bearer token authentication
- Compile-time guarantees for memory safety

## Performance Optimizations

1. **Zero-Copy Operations**: Minimize allocations with borrowing and references
2. **Lazy Initialization**: OnceCell patterns for model, cache, config
3. **Efficient Serialization**: Binary format for embedding cache with serde
4. **Connection Pooling**: Both database and Redis with async pools
5. **Lock Granularity**: Fine-grained RwLock for caching
6. **SIMD Operations**: Leverage hardware acceleration where available

## Differences from Python Version

| Aspect | Python | Rust |
|--------|--------|------|
| HTTP Framework | FastAPI | Axum |
| Async Model | AsyncIO | Tokio |
| ORM | SQLAlchemy | Raw SQL (tokio-postgres) |
| Tokenizer | transformers | Custom implementation |
| Cache | External library | Custom LRU |
| Type Safety | Runtime (Pydantic) | Compile-time |
| Memory Safety | Runtime (GC) | Compile-time (borrow checker) |

## Dependencies

Main external dependencies:
- `tokio-postgres` - PostgreSQL driver
- `redis` - Redis client
- `ort` - ONNX Runtime bindings
- `axum` - HTTP server framework
- `prometheus` - Metrics
- `bcrypt` - Password hashing
- `serde` - Serialization framework

## Building and Running

```bash
# Build
make build

# Run
make run

# Or directly
cargo build --release
./target/release/embed_rs
```

## Configuration

Environment variables are read via the `config` module:
- Uses `std::env::var()` with defaults
- OnceCell pattern ensures one config instance
- Type-safe accessors for all settings
- Compile-time validation where possible

## Testing Strategy

1. Unit tests for core logic (cache, tokenizer, security)
2. Integration tests for database operations
3. Benchmark tests with criterion for performance validation
4. Load testing with external tools (wrk, k6)

## Future Improvements

1. **Connection Pooling**: Tune pool sizes based on load
2. **Batch Processing**: Support multiple embeddings per request
3. **Model Hot-Reload**: Support model updates without restart
4. **Distributed Caching**: Add cache sharding for scale
5. **OpenTelemetry**: Add distributed tracing
6. **gRPC Support**: Alternative to REST API

## Monitoring

Prometheus metrics exposed at `/metrics`:
- Request latency histograms
- Cache hit/miss counters
- Error counters by type
- Rate limit exceeded counters
- Token count distributions

## Deployment

Recommended deployment:
```bash
# Build static binary for production
cargo build --release

# Cross-compile for Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# Docker
docker build -t fastembed-rs .
docker run -p 8000:8000 --env-file .env fastembed-rs
```

## Maintenance

- Run `cargo update` to update dependencies
- Format code with `cargo fmt`
- Lint with `cargo clippy`
- Check compilation with `cargo check`
- Security audit with `cargo audit`
