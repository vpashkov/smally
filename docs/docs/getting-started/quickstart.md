---
sidebar_position: 2
---

# Quick Start

Get up and running with Smally in under 5 minutes.

## 1. Create an API Key

First, you'll need to create an API key. For development, use the CLI tool:

```bash
cargo run --bin create_api_key -- \
  --user-id 1 \
  --org-id 1 \
  --name "My First Key"
```

This will output your API key:

```
API Key created successfully!
Key: sk_abc123def456...
```

Save this key - you won't be able to see it again!

## 2. Make Your First Request

Use your API key to create an embedding:

```bash
curl -X POST "http://localhost:8000/v1/embed" \
  -H "Authorization: Bearer sk_abc123def456..." \
  -H "Content-Type: application/json" \
  -d '{
    "text": "The quick brown fox jumps over the lazy dog",
    "normalize": false
  }'
```

Response:

```json
{
  "embedding": [
    0.0234, -0.1567, 0.0892, ...
  ],
  "tokens": 10,
  "cached": false,
  "model": "all-MiniLM-L6-v2"
}
```

## 3. Use the Interactive Swagger UI

Visit the interactive API documentation:

```
http://localhost:8000/swagger-ui
```

1. Click **Authorize** in the top right
2. Enter your API key
3. Click **Authorize** then **Close**
4. Try the **POST /v1/embed** endpoint

## Understanding the Response

```json
{
  "embedding": [...],  // 384-dimensional vector
  "tokens": 10,        // Number of tokens in input
  "cached": false,     // Whether result was cached
  "model": "all-MiniLM-L6-v2"  // Model used
}
```

### Response Headers

Rate limit information is included in the response headers:

```
X-RateLimit-Limit: 20000
X-RateLimit-Remaining: 19999
X-RateLimit-Reset: 2025-02-01T00:00:00Z
```

## Code Examples

### Python

```python
import requests

response = requests.post(
    'http://localhost:8000/v1/embed',
    headers={
        'Authorization': 'Bearer sk_abc123def456...',
        'Content-Type': 'application/json'
    },
    json={
        'text': 'Hello world',
        'normalize': False
    }
)

data = response.json()
print(f"Embedding: {data['embedding'][:5]}...")  # First 5 dimensions
print(f"Tokens: {data['tokens']}")
print(f"Cached: {data['cached']}")
```

### JavaScript

```javascript
const response = await fetch('http://localhost:8000/v1/embed', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer sk_abc123def456...'
  },
  body: JSON.stringify({
    text: 'Hello world',
    normalize: false
  })
});

const data = await response.json();
console.log('Embedding:', data.embedding.slice(0, 5), '...');
console.log('Tokens:', data.tokens);
console.log('Cached:', data.cached);
```

### Rust

```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8000/v1/embed")
        .header("Authorization", "Bearer sk_abc123def456...")
        .json(&json!({
            "text": "Hello world",
            "normalize": false
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    println!("Embedding: {:?}", response["embedding"]);
    println!("Tokens: {}", response["tokens"]);
    println!("Cached: {}", response["cached"]);

    Ok(())
}
```

## Next Steps

- [Authentication](/docs/getting-started/authentication) - Learn about API keys and security
- [Embedding Text](/docs/guides/embedding-text) - Advanced usage patterns
- [API Reference](/api) - Full API documentation
