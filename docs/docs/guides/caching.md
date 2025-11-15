---
sidebar_position: 2
---

# Caching

Smally uses Redis-backed caching to provide lightning-fast responses for repeated queries.

## How Caching Works

When you make an embedding request:

1. **Cache lookup**: Smally checks if this exact text has been embedded before
2. **Cache hit**: If found, return the cached embedding (~1ms)
3. **Cache miss**: If not found, compute embedding and cache it

```bash
# First request - cache miss
curl -X POST http://localhost:8000/v1/embed \
  -H "Authorization: Bearer YOUR_KEY" \
  -d '{"text": "Hello world"}'

# Response: { ..., "cached": false }
# Latency: ~10ms

# Second request - cache hit!
curl -X POST http://localhost:8000/v1/embed \
  -H "Authorization: Bearer YOUR_KEY" \
  -d '{"text": "Hello world"}'

# Response: { ..., "cached": true }
# Latency: ~1ms
```

## Cache Key Generation

The cache key is based on:

1. **Text content**: Exact string match (case-sensitive)
2. **Model version**: Different models have separate caches
3. **Cache version**: Allows cache invalidation

```python
# These are cached separately:
embed("Hello World")   # Different from below
embed("hello world")   # Different case

embed("Hello World", normalize=True)   # Normalization is applied after cache lookup
embed("Hello World", normalize=False)  # Same cache entry, different processing
```

## Cache Storage

### What's Cached

For each unique text input, we store:

```rust
{
  "embedding": Vec<f32>,  // 384-dimensional vector
  "tokens": usize,        // Token count
  "model": String,        // Model identifier
}
```

### Serialization

- **Format**: Bincode (compact binary)
- **Size**: ~1.5 KB per entry (384 floats + metadata)
- **Compression**: None (Redis handles compression if enabled)

## Cache Performance

### Hit Rates

Typical cache hit rates by use case:

| Use Case | Hit Rate | Benefit |
|----------|----------|---------|
| FAQ/Support | 80-95% | Very high - limited question set |
| Search queries | 40-60% | High - common queries repeated |
| Document processing | 5-15% | Low - mostly unique content |

### Latency Comparison

```
Cache miss:  ~10ms (ONNX inference)
Cache hit:   ~1ms  (Redis lookup)
Speedup:     10x faster
```

## Cache Management

### Cache TTL

Currently, cache entries never expire (infinite TTL). Future versions may add:

- Time-based expiration (e.g., 30 days)
- LRU eviction when memory limit reached
- Manual cache invalidation API

### Monitoring Cache

Check cache effectiveness:

```bash
# Get cache stats from Redis
redis-cli INFO stats

# Key metrics:
# - keyspace_hits: Cache hits
# - keyspace_misses: Cache misses
# - used_memory: Total cache size
```

### Cache Hit Rate

```python
hit_rate = keyspace_hits / (keyspace_hits + keyspace_misses)

# Good: > 50%
# Excellent: > 80%
```

## Optimizing Cache Usage

### 1. Normalize Input

Pre-process text to maximize cache hits:

```python
def normalize_text(text):
    # Lowercase
    text = text.lower()
    # Remove extra whitespace
    text = ' '.join(text.split())
    # Remove punctuation (if appropriate)
    # text = text.translate(str.maketrans('', '', string.punctuation))
    return text

# More cache hits!
embed(normalize_text("  Hello World!  "))  # "hello world"
embed(normalize_text("hello world"))       # Same cache entry
```

### 2. Deduplicate Before Embedding

Avoid redundant API calls:

```python
texts = ["hello", "world", "hello", "foo", "world"]

# Bad: 5 API calls (but 2 are cached)
embeddings = [embed(text) for text in texts]

# Good: 3 API calls
unique_texts = list(set(texts))
unique_embeddings = {text: embed(text) for text in unique_texts}
embeddings = [unique_embeddings[text] for text in texts]
```

### 3. Warm Up Cache

Pre-populate cache with common queries:

```python
common_queries = [
    "how to reset password",
    "contact support",
    "pricing information",
    # ...
]

# Warm up cache
for query in common_queries:
    embed(query)
```

## Cache Invalidation

### When to Invalidate

Invalidate the cache when:

- Upgrading to a new model version
- Fixing a bug in embedding logic
- Changing tokenization

### How to Invalidate

#### Option 1: Flush Redis

```bash
# Warning: Deletes ALL cache entries
redis-cli FLUSHDB
```

#### Option 2: Bump Cache Version

Update the cache version in your `.env`:

```bash
# Old cache becomes inaccessible
CACHE_VERSION=2
```

This keeps old data but creates a new namespace.

## Cache Costs

### Storage

```
1 million cached embeddings:
- 384 floats × 4 bytes = 1,536 bytes per embedding
- + metadata (~100 bytes)
- ≈ 1.6 GB total

Redis memory: ~2 GB with overhead
```

### Redis Configuration

For production, configure Redis for optimal caching:

```bash
# redis.conf

# Max memory (adjust based on needs)
maxmemory 4gb

# Eviction policy
maxmemory-policy allkeys-lru

# Persistence (optional, for cache recovery)
save 900 1
save 300 10
```

## Advanced: Cache Warming Strategy

For production systems with known query patterns:

```python
import schedule
import time

def warm_cache():
    """Refresh top 1000 queries daily"""
    top_queries = get_top_queries(limit=1000)

    for query in top_queries:
        try:
            embed(query)
        except Exception as e:
            print(f"Failed to cache '{query}': {e}")

# Run daily at 2 AM
schedule.every().day.at("02:00").do(warm_cache)

while True:
    schedule.run_pending()
    time.sleep(60)
```

## Troubleshooting

### Low Cache Hit Rate

**Problem**: Cache hit rate < 20%

**Possible causes:**
1. Mostly unique content (expected for document processing)
2. Input text not normalized (case, whitespace variations)
3. Cache was recently cleared

**Solutions:**
- Normalize input text
- Warm up cache with common queries
- Analyze query patterns

### Cache Connection Errors

**Problem**: `ERR_CACHE_UNAVAILABLE`

**Solutions:**
```bash
# Check Redis is running
redis-cli ping

# Check connection settings
echo $REDIS_URL

# Restart Redis
brew services restart redis  # macOS
sudo systemctl restart redis # Linux
```

## Next Steps

- [Rate Limits](/docs/guides/rate-limits) - Monitor API usage
- [API Reference](/api) - Full API documentation
