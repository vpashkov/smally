---
sidebar_position: 1
---

# API Overview

The Smally API provides a simple REST interface for creating text embeddings.

## Base URL

```
http://localhost:8000  # Development
https://api.smally.ai  # Production
```

## Authentication

All API endpoints require authentication via Bearer token:

```http
POST /v1/embed
Authorization: Bearer sk_YOUR_API_KEY
Content-Type: application/json
```

See [Authentication](/docs/getting-started/authentication) for details.

## Endpoints

### POST /v1/embed

Create text embeddings.

**Request:**

```json
{
  "text": "Your text here",
  "normalize": false
}
```

**Response:**

```json
{
  "embedding": [0.123, -0.456, ...],
  "tokens": 5,
  "cached": false,
  "model": "all-MiniLM-L6-v2"
}
```

**Rate Limited**: Yes
**Cached**: Yes

[Full API Reference →](/api)

### GET /health

Check API health and version.

**Response:**

```json
{
  "status": "ok",
  "version": "0.1.0",
  "build": {
    "version": "0.1.0",
    "commit": "abc123",
    "date": "2025-01-15",
    "rustc": "1.82.0"
  }
}
```

**Rate Limited**: No
**Cached**: No

### GET /api

Get API information.

**Response:**

```json
{
  "name": "Smally Embeddings API",
  "version": "0.1.0",
  "docs": "http://localhost:8000/swagger-ui"
}
```

**Rate Limited**: No
**Cached**: No

## Request Format

### Headers

```http
Authorization: Bearer YOUR_API_KEY
Content-Type: application/json
Accept: application/json
```

### Body

JSON-encoded request body:

```json
{
  "text": "string",
  "normalize": boolean
}
```

## Response Format

### Success Response

```http
HTTP/1.1 200 OK
Content-Type: application/json
X-RateLimit-Limit: 100000
X-RateLimit-Remaining: 99999
X-RateLimit-Reset: 2025-02-01T00:00:00Z

{
  "embedding": [...],
  "tokens": 5,
  "cached": false,
  "model": "all-MiniLM-L6-v2"
}
```

### Error Response

```http
HTTP/1.1 400 Bad Request
Content-Type: application/json

{
  "error": "invalid_request",
  "message": "Text cannot be empty"
}
```

See [Error Handling](/docs/api/errors) for all error types.

## Rate Limiting

All endpoints (except `/health` and `/api`) are rate limited.

**Headers:**

- `X-RateLimit-Limit`: Monthly quota
- `X-RateLimit-Remaining`: Requests remaining
- `X-RateLimit-Reset`: Reset timestamp

See [Rate Limits](/docs/guides/rate-limits) for details.

## Caching

The `/v1/embed` endpoint caches results automatically:

- **Cache key**: Text content + model version
- **Cache backend**: Redis
- **TTL**: Infinite (currently)

The `cached` field in the response indicates if the result was cached.

See [Caching](/docs/guides/caching) for details.

## Versioning

The API uses URL versioning:

- **Current**: `/v1/*`
- **Future**: `/v2/*` (when available)

Breaking changes will be released in new versions. Existing versions remain supported.

## CORS

CORS is enabled for all origins in development:

```http
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization, Accept
```

Production CORS settings can be configured.

## OpenAPI Specification

Interactive API documentation:

- **Swagger UI**: `http://localhost:8000/swagger-ui`
- **OpenAPI JSON**: `http://localhost:8000/openapi.json`

## Client Libraries

Generate client libraries from the OpenAPI spec:

```bash
# Download spec
curl http://localhost:8000/openapi.json > openapi.json

# Generate TypeScript client
openapi-generator-cli generate \
  -i openapi.json \
  -g typescript-axios \
  -o ./client

# Generate Python client
openapi-generator-cli generate \
  -i openapi.json \
  -g python \
  -o ./client
```

## SDKs

Official SDKs (coming soon):

- Python: `pip install smally`
- JavaScript/TypeScript: `npm install @smally/client`
- Rust: `cargo add smally-client`

## Webhooks

Webhooks for events (coming soon):

- `embedding.created`
- `rate_limit.warning`
- `rate_limit.exceeded`

## Next Steps

- [Error Handling](/docs/api/errors) - Understanding API errors
- [Full API Reference](/api) - Interactive Swagger documentation
- [Authentication](/docs/getting-started/authentication) - API keys and security
