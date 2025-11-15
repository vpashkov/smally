---
sidebar_position: 2
---

# Error Handling

Smally uses conventional HTTP status codes and returns structured error responses.

## Error Response Format

All errors follow this structure:

```json
{
  "error": "error_type",
  "message": "Human-readable error description"
}
```

## HTTP Status Codes

| Status | Meaning |
|--------|---------|
| **200** | Success |
| **400** | Bad Request - Invalid input |
| **401** | Unauthorized - Invalid/missing API key |
| **429** | Too Many Requests - Rate limit exceeded |
| **500** | Internal Server Error |
| **503** | Service Unavailable - Temporary outage |

## Error Types

### `invalid_request` (400)

The request body is malformed or contains invalid parameters.

**Example:**

```json
{
  "error": "invalid_request",
  "message": "Text cannot be empty"
}
```

**Common causes:**

- Empty `text` field
- Invalid JSON
- Missing required fields
- Wrong data types

**Solutions:**

```python
# ✅ Good
{
  "text": "Hello world",
  "normalize": false
}

# ❌ Bad - empty text
{
  "text": "",
  "normalize": false
}

# ❌ Bad - missing text
{
  "normalize": false
}

# ❌ Bad - wrong type
{
  "text": 123,  # Should be string
  "normalize": "yes"  # Should be boolean
}
```

### `text_too_long` (400)

Input text exceeds maximum token limit (128 tokens).

**Example:**

```json
{
  "error": "text_too_long",
  "message": "Text exceeds maximum token limit of 128"
}
```

**Solutions:**

```python
# Option 1: Truncate text
text = long_text[:500]  # Rough approximation

# Option 2: Split into chunks
def chunk_text(text, max_chars=500):
    words = text.split()
    chunks = []
    current = []

    for word in words:
        current.append(word)
        if len(' '.join(current)) > max_chars:
            chunks.append(' '.join(current[:-1]))
            current = [word]

    if current:
        chunks.append(' '.join(current))

    return chunks

# Embed each chunk
chunks = chunk_text(long_text)
embeddings = [embed(chunk) for chunk in chunks]
```

### `invalid_api_key` (401)

API key is missing, invalid, or revoked.

**Example:**

```json
{
  "error": "invalid_api_key",
  "message": "Invalid API key"
}
```

**Common causes:**

- Missing `Authorization` header
- Malformed Bearer token
- Revoked API key
- Expired token
- Wrong API key for environment

**Solutions:**

```python
# ✅ Good
headers = {
    'Authorization': 'Bearer sk_abc123...'
}

# ❌ Bad - missing Authorization
headers = {
    'Content-Type': 'application/json'
}

# ❌ Bad - wrong format
headers = {
    'Authorization': 'sk_abc123...'  # Missing "Bearer"
}

# ❌ Bad - using wrong key
headers = {
    'Authorization': f'Bearer {DEV_KEY}'  # In production
}
```

### `rate_limit_exceeded` (429)

Monthly quota exhausted.

**Example:**

```json
{
  "error": "rate_limit_exceeded",
  "message": "Monthly quota exhausted. Resets on 2025-02-01T00:00:00Z"
}
```

**Response headers:**

```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 10000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 2025-02-01T00:00:00Z
Retry-After: 86400
```

**Solutions:**

```python
# Option 1: Wait for reset
import time
retry_after = int(response.headers['Retry-After'])
time.sleep(retry_after)

# Option 2: Upgrade tier
# Visit: http://localhost:8000/billing

# Option 3: Use caching more effectively
# Identical requests are free!
```

See [Rate Limits](/docs/guides/rate-limits) for details.

### `internal_error` (500)

Unexpected server error.

**Example:**

```json
{
  "error": "internal_error",
  "message": "Internal server error"
}
```

**Common causes:**

- Database connection failure
- Redis connection failure
- Model loading error
- Unexpected exception

**Solutions:**

```python
# Implement retry logic with exponential backoff
import time

def embed_with_retry(text, max_retries=3):
    for attempt in range(max_retries):
        try:
            response = requests.post(...)
            response.raise_for_status()
            return response.json()
        except requests.exceptions.HTTPError as e:
            if e.response.status_code == 500:
                if attempt < max_retries - 1:
                    wait = 2 ** attempt  # Exponential backoff
                    print(f"Server error. Retrying in {wait}s...")
                    time.sleep(wait)
                else:
                    raise
            else:
                raise
```

If errors persist, check:

- Server logs
- Database connectivity
- Redis connectivity
- System resources (CPU, memory)

## Error Handling Best Practices

### 1. Always Check Status Codes

```python
response = requests.post(...)

if response.status_code == 200:
    data = response.json()
    # Process success
elif response.status_code == 400:
    error = response.json()
    print(f"Invalid request: {error['message']}")
elif response.status_code == 401:
    print("Invalid API key")
elif response.status_code == 429:
    print("Rate limited")
else:
    print(f"Unexpected error: {response.status_code}")
```

### 2. Parse Error Messages

```python
def handle_error(response):
    try:
        error = response.json()
        error_type = error.get('error', 'unknown')
        message = error.get('message', 'Unknown error')

        if error_type == 'rate_limit_exceeded':
            # Handle rate limit
            reset = response.headers.get('X-RateLimit-Reset')
            print(f"Rate limited. Resets: {reset}")
        elif error_type == 'invalid_request':
            # Handle invalid input
            print(f"Invalid input: {message}")
        else:
            print(f"Error: {message}")

    except ValueError:
        # Response is not JSON
        print(f"HTTP {response.status_code}: {response.text}")
```

### 3. Implement Retry Logic

```python
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

# Retry on 500, 502, 503, 504
retry_strategy = Retry(
    total=3,
    status_forcelist=[500, 502, 503, 504],
    backoff_factor=1
)

adapter = HTTPAdapter(max_retries=retry_strategy)
session = requests.Session()
session.mount("http://", adapter)
session.mount("https://", adapter)

# Automatic retries on 5xx errors
response = session.post(...)
```

### 4. Monitor Error Rates

```python
from collections import Counter

error_counter = Counter()

def track_error(error_type):
    error_counter[error_type] += 1

    # Alert on high error rates
    total = sum(error_counter.values())
    rate = error_counter[error_type] / total

    if rate > 0.1:  # > 10% errors
        print(f"⚠️  High {error_type} rate: {rate*100:.1f}%")
```

### 5. Graceful Degradation

```python
def embed_with_fallback(text, fallback=None):
    try:
        return embed(text)
    except RateLimitError:
        print("Rate limited. Using fallback.")
        return fallback or [0.0] * 384  # Zero vector
    except Exception as e:
        print(f"Error: {e}. Using fallback.")
        return fallback or [0.0] * 384
```

## Debugging Errors

### Enable Verbose Logging

```python
import logging
import http.client

# Log all HTTP requests/responses
http.client.HTTPConnection.debuglevel = 1

logging.basicConfig()
logging.getLogger().setLevel(logging.DEBUG)
requests_log = logging.getLogger("requests.packages.urllib3")
requests_log.setLevel(logging.DEBUG)
requests_log.propagate = True

# Make request
response = requests.post(...)
```

### Check Request Details

```python
import requests

# Prepare request
req = requests.Request(
    'POST',
    'http://localhost:8000/v1/embed',
    headers={'Authorization': 'Bearer ...'},
    json={'text': 'test', 'normalize': False}
)

prepared = req.prepare()

# Inspect before sending
print("URL:", prepared.url)
print("Headers:", prepared.headers)
print("Body:", prepared.body)

# Send
session = requests.Session()
response = session.send(prepared)
```

### Validate JSON

```python
import json

try:
    data = {'text': 'test', 'normalize': False}
    json_str = json.dumps(data)
    print("Valid JSON:", json_str)
except ValueError as e:
    print("Invalid JSON:", e)
```

## Production Error Monitoring

Set up error tracking in production:

```python
import sentry_sdk

sentry_sdk.init(dsn="YOUR_SENTRY_DSN")

try:
    response = embed("text")
except Exception as e:
    # Auto-reported to Sentry
    sentry_sdk.capture_exception(e)
    raise
```

## Getting Help

If you encounter persistent errors:

1. Check [API Status](https://status.smally.ai)
2. Review [Documentation](/docs/intro)
3. Search [GitHub Issues](https://github.com/your-org/smally/issues)
4. Contact [Support](mailto:support@smally.ai)

## Next Steps

- [API Overview](/docs/api/overview) - Complete API reference
- [Authentication](/docs/getting-started/authentication) - Fix auth errors
- [Rate Limits](/docs/guides/rate-limits) - Handle rate limiting
