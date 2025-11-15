---
sidebar_position: 3
---

# Authentication

Smally uses API keys with CBOR Web Tokens (CWT) for secure authentication.

## API Keys

API keys are organization-scoped tokens that authenticate your requests to the Smally API.

### Key Format

All API keys use the following format:

```
fe_<hash>
```

- **Prefix**: `fe_` identifies this as a Smally API key
- **Hash**: Cryptographically secure random string

Example: `fe_33c3842b326b85e8a50485ea0a5ad72eb66d68694f0bed52e0fd923813ec1ed9`

### Creating API Keys

#### Using the CLI

```bash
cargo run --bin create_api_key -- \
  --user-id <USER_ID> \
  --org-id <ORG_ID> \
  --name "Production API Key"
```

#### Using the Web UI

1. Log in to your dashboard at `http://localhost:8000/dashboard`
2. Navigate to your organization
3. Click "Create API Key"
4. Give it a descriptive name
5. Copy the key (you won't see it again!)

### Using API Keys

Include your API key in the `Authorization` header:

```bash
curl -H "Authorization: Bearer fe_YOUR_API_KEY" \
  http://localhost:8000/v1/embed
```

The `fe_` prefix is optional - the API strips it automatically:

```bash
# Both work the same
Authorization: Bearer fe_abc123...
Authorization: Bearer abc123...
```

## Security Best Practices

### Keep Keys Secret

- Never commit API keys to version control
- Use environment variables in production
- Rotate keys regularly

```bash
# Good: Use environment variable
export SMALLY_API_KEY="fe_abc123..."
curl -H "Authorization: Bearer $SMALLY_API_KEY" ...

# Bad: Hardcoded in script
curl -H "Authorization: Bearer fe_abc123..." ...
```

### Use Different Keys for Different Environments

```bash
# Development
SMALLY_API_KEY_DEV="fe_dev_key..."

# Staging
SMALLY_API_KEY_STAGING="fe_staging_key..."

# Production
SMALLY_API_KEY_PROD="fe_prod_key..."
```

### Revoke Compromised Keys

If a key is compromised, revoke it immediately:

```bash
# Via Web UI
1. Go to organization page
2. Find the compromised key
3. Click "Revoke"

# Via CLI
cargo run --bin create_api_key -- revoke --key-id <KEY_ID>
```

## Token Details (CWT)

Under the hood, Smally uses CBOR Web Tokens (CWT) for authentication:

- **CBOR**: Compact binary serialization
- **Ed25519**: Cryptographic signatures
- **Stateless**: No server-side session storage

### Token Claims

Each token contains:

```rust
{
  "key_id": "uuid-of-api-key",
  "org_id": "uuid-of-organization",
  "tier": "free" | "pro" | "enterprise",
  "exp": timestamp
}
```

### Token Validation

Tokens are validated on every request:

1. Signature verification using Ed25519 public key
2. Expiration check
3. Key status check (active/revoked)
4. Rate limit check based on tier

## Rate Limiting

Rate limits are enforced per organization based on subscription tier:

| Tier | Monthly Requests |
|------|-----------------|
| Free | 10,000 |
| Pro | 100,000 |
| Enterprise | Unlimited |

Rate limit info is returned in response headers:

```
X-RateLimit-Limit: 100000
X-RateLimit-Remaining: 95432
X-RateLimit-Reset: 2025-02-01T00:00:00Z
```

When you exceed your rate limit, you'll receive a `429 Too Many Requests` error:

```json
{
  "error": "rate_limit_exceeded",
  "message": "Monthly quota exhausted"
}
```

## Error Responses

### 401 Unauthorized

Invalid or missing API key:

```json
{
  "error": "invalid_api_key",
  "message": "Invalid API key"
}
```

Common causes:
- Missing `Authorization` header
- Invalid key format
- Revoked key
- Expired token

### 429 Rate Limit Exceeded

```json
{
  "error": "rate_limit_exceeded",
  "message": "Monthly quota exhausted"
}
```

Check the `X-RateLimit-Reset` header to see when your quota resets.

## Next Steps

- [Embedding Text](/docs/guides/embedding-text) - Learn how to use the API
- [Rate Limits](/docs/guides/rate-limits) - Understanding rate limits
- [API Reference](/api) - Complete API documentation
