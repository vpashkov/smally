---
sidebar_position: 1
slug: /
---

# Introduction

Welcome to **Smally** - a fast, production-ready text embedding API powered by sentence transformers.

## What is Smally?

Smally provides high-performance text embeddings through a simple REST API. Built with Rust and ONNX Runtime, it offers:

- **Fast**: Sub-10ms inference with ONNX optimization
- **Cached**: Redis-backed caching for instant responses
- **Scalable**: Production-ready with rate limiting and monitoring
- **Simple**: Clean REST API with OpenAPI documentation

## Key Features

### 🚀 High Performance

- ONNX Runtime for optimized inference
- Redis caching with sub-millisecond lookups
- Connection pooling for database and cache

### 🔒 Production Ready

- JWT-based authentication
- API key management
- Rate limiting by organization tier
- Comprehensive error handling

### 📊 Monitoring

- Prometheus metrics
- Request/response logging
- Usage tracking and analytics

### 📖 Developer Friendly

- Interactive OpenAPI/Swagger documentation
- Clear error messages
- Code examples in multiple languages

## Quick Example

```bash
curl -X POST "http://localhost:8000/v1/embed" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello world", "normalize": false}'
```

Response:

```json
{
  "embedding": [0.123, -0.456, ...],
  "tokens": 3,
  "cached": false,
  "model": "all-MiniLM-L6-v2"
}
```

## Next Steps

- [Installation](/docs/getting-started/installation) - Set up Smally locally
- [Quick Start](/docs/getting-started/quickstart) - Create your first embeddings
- [API Reference](/api) - Explore the full API documentation
