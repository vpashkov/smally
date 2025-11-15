---
sidebar_position: 3
---

# Rate Limits

Smally enforces rate limits based on your organization's subscription tier to ensure fair usage and system stability.

## Rate Limit Tiers

| Tier | Monthly Requests | Cost |
|------|-----------------|------|
| **Free** | 10,000 | $0 |
| **Pro** | 100,000 | $29/mo |
| **Scale** | 1,000,000 | $199/mo |
| **Enterprise** | Unlimited | Custom |

## How Rate Limiting Works

Rate limits are enforced **per organization** and reset monthly:

1. Each API request decrements your remaining quota
2. Quota resets on the 1st of each month at 00:00 UTC
3. All API keys in an organization share the same quota

```
Organization "Acme Corp" (Pro tier):
├─ API Key 1: "Production"     ╲
├─ API Key 2: "Staging"         ├─ Share 100,000/month
└─ API Key 3: "Development"    ╱
```

## Checking Your Rate Limit

### Response Headers

Every API response includes rate limit headers:

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100000
X-RateLimit-Remaining: 95432
X-RateLimit-Reset: 2025-02-01T00:00:00Z
```

- **`X-RateLimit-Limit`**: Total monthly quota
- **`X-RateLimit-Remaining`**: Requests left this month
- **`X-RateLimit-Reset`**: When quota resets (ISO 8601)

### Parsing Headers

```python
import requests
from datetime import datetime

response = requests.post(...)

limit = int(response.headers['X-RateLimit-Limit'])
remaining = int(response.headers['X-RateLimit-Remaining'])
reset = datetime.fromisoformat(
    response.headers['X-RateLimit-Reset'].replace('Z', '+00:00')
)

print(f"Used: {limit - remaining}/{limit}")
print(f"Resets: {reset}")
```

### Dashboard

View usage in real-time:

```
http://localhost:8000/dashboard
```

- Current usage
- Historical trends
- Per-key breakdown
- Usage alerts

## Rate Limit Exceeded

When you exceed your quota, requests return `429 Too Many Requests`:

```json
{
  "error": "rate_limit_exceeded",
  "message": "Monthly quota exhausted. Resets on 2025-02-01T00:00:00Z"
}
```

### Error Response

```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 10000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 2025-02-01T00:00:00Z
Retry-After: 86400

{
  "error": "rate_limit_exceeded",
  "message": "Monthly quota exhausted"
}
```

- **`Retry-After`**: Seconds until quota reset

### Handling Rate Limits

```python
import time

def embed_with_retry(text, max_retries=3):
    for attempt in range(max_retries):
        response = requests.post(...)

        if response.status_code == 200:
            return response.json()

        if response.status_code == 429:
            # Rate limited
            retry_after = int(response.headers.get('Retry-After', 60))

            if attempt < max_retries - 1:
                print(f"Rate limited. Waiting {retry_after}s...")
                time.sleep(retry_after)
            else:
                raise Exception("Rate limit exceeded")

        else:
            response.raise_for_status()

    raise Exception("Max retries exceeded")
```

## Optimizing Rate Limit Usage

### 1. Leverage Caching

Identical requests are cached and don't count toward rate limits:

```python
# First call: Uses 1 quota
embed("common query")  # cached: false

# Second call: FREE! (cached)
embed("common query")  # cached: true
```

**Impact**: Can reduce quota usage by 50-80% for typical workloads.

### 2. Deduplicate Requests

```python
# Bad: 1000 requests (many duplicates)
texts = ["hello"] * 500 + ["world"] * 500
for text in texts:
    embed(text)  # Uses 1000 quota

# Good: 2 requests + caching
unique_texts = set(texts)
embeddings = {text: embed(text) for text in unique_texts}
# Uses 2 quota, rest are cached
```

### 3. Batch Processing

Process in batches to monitor usage:

```python
def embed_batch(texts, batch_size=100):
    results = []

    for i in range(0, len(texts), batch_size):
        batch = texts[i:i+batch_size]

        # Check rate limit before batch
        response = requests.post(...)
        remaining = int(response.headers['X-RateLimit-Remaining'])

        if remaining < batch_size:
            print(f"Warning: Only {remaining} requests left")
            # Maybe wait or upgrade tier

        results.extend([embed(text) for text in batch])

    return results
```

### 4. Monitor Usage

Set up alerts when approaching limits:

```python
def check_rate_limit_warning(response, threshold=0.9):
    limit = int(response.headers['X-RateLimit-Limit'])
    remaining = int(response.headers['X-RateLimit-Remaining'])
    used_pct = (limit - remaining) / limit

    if used_pct >= threshold:
        print(f"⚠️  WARNING: {used_pct*100:.1f}% of quota used!")
        # Send alert, upgrade tier, etc.
```

## Rate Limit Best Practices

### Development vs Production

Use different API keys and organizations:

```python
# Development (Free tier - 10k/month)
DEV_API_KEY = "sk_dev_..."

# Production (Pro tier - 100k/month)
PROD_API_KEY = "sk_prod_..."
```

This prevents dev/testing from consuming production quota.

### Estimate Usage

Before deploying, estimate monthly usage:

```python
# Example calculation
requests_per_user_per_day = 50
active_users = 100
days_per_month = 30

monthly_requests = (
    requests_per_user_per_day *
    active_users *
    days_per_month
)  # = 150,000

# Need Pro tier (100k) or Scale tier (1M)
```

### Implement Backoff

When approaching limits, slow down requests:

```python
def adaptive_embed(text):
    response = requests.post(...)

    remaining = int(response.headers['X-RateLimit-Remaining'])
    limit = int(response.headers['X-RateLimit-Limit'])

    # Slow down when < 10% remaining
    if remaining < limit * 0.1:
        time.sleep(1)  # Add delay

    return response.json()
```

## Upgrading Your Tier

### When to Upgrade

Upgrade when you consistently:

- Hit rate limits before month end
- Need higher throughput
- Want production SLAs

### How to Upgrade

```bash
# Contact sales for tier upgrade
# Or visit dashboard
http://localhost:8000/dashboard/billing
```

Changes take effect immediately.

## Enterprise Custom Limits

Enterprise tier offers:

- **Unlimited requests**
- **Custom rate limits** (e.g., per-second instead of per-month)
- **Dedicated infrastructure**
- **SLA guarantees**
- **Priority support**

Contact sales: <sales@smally.ai>

## Troubleshooting

### Unexpected Rate Limit

**Problem**: Hit rate limit earlier than expected

**Possible causes:**

1. **Multiple API keys**: All keys in org share quota

   ```bash
   # Check all keys in your organization
   curl http://localhost:8000/v1/organizations/ORG_ID/keys
   ```

2. **Uncached requests**: Not leveraging cache effectively

   ```python
   # Add logging to check cache hit rate
   if response['cached']:
       cache_hits += 1
   ```

3. **Testing in production**: Dev/test using production keys

   ```python
   # Use separate keys
   if ENV == 'development':
       api_key = DEV_KEY  # Free tier
   else:
       api_key = PROD_KEY  # Pro tier
   ```

### Rate Limit Not Resetting

**Problem**: Limit didn't reset on 1st of month

**Solutions:**

```bash
# Check server time (must be UTC)
curl http://localhost:8000/health

# Verify organization tier
curl http://localhost:8000/v1/organizations/ORG_ID
```

Contact support if issue persists.

## Next Steps

- [Caching](/docs/guides/caching) - Reduce quota usage with caching
- [API Reference](/api) - Full API documentation
- [Dashboard](http://localhost:8000/dashboard) - Monitor usage
