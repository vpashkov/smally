# Event Sourcing Architecture for Recalculable Usage

## Overview

This system implements an event sourcing pattern that allows you to recalculate usage and billing at any time, even if there are bugs in the token counting algorithm or changes to pricing models.

## Architecture

### Two-Table Design

**1. `api_request_log` - Source of Truth (Immutable Event Store)**
- Contains **every API request** with the original input text
- Allows recalculation of tokens if the counting algorithm changes
- Tracks failed requests (no response data = failure)
- Never delete or update (append-only except for response updates)

**2. `usage_events` - Derived Billing Data (Recalculable)**
- Aggregated usage events for fast billing queries
- Can be completely deleted and recalculated from `api_request_log`
- Only contains successful requests (those with responses)

### Request Flow

```
1. Request arrives
   ↓
2. record_request() → INSERT to api_request_log (immediate, non-blocking)
   - Status: 'pending'
   - Stores: input_text, organization_id, api_key_id, etc.
   ↓
3. Process request (count tokens, generate embedding, etc.)
   ↓
4. record_response() → UPDATE api_request_log + INSERT to usage_events (buffered)
   - Status: 'success'
   - Updates: tokens, response_metadata, response_timestamp
   - Inserts billing event to usage_events
   ↓
5. Background flush task (every 5 seconds)
   - Batch UPDATE api_request_log
   - Batch INSERT usage_events
```

## Schema

```sql
-- Source of truth
CREATE TABLE api_request_log (
    request_id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    api_key_id UUID NOT NULL,

    -- Product info
    product VARCHAR(50) NOT NULL,          -- 'embeddings', 'rerank', etc.
    endpoint VARCHAR(100) NOT NULL,        -- '/v1/embed', '/v1/rerank'

    -- Request data (filled immediately)
    input_text TEXT NOT NULL,              -- Original input for recalculation
    input_metadata JSONB,
    request_timestamp TIMESTAMP NOT NULL,

    -- Response data (filled on success)
    tokens INTEGER,                         -- Can be recalculated from input_text
    response_metadata JSONB,
    response_timestamp TIMESTAMP,

    status VARCHAR(50) DEFAULT 'pending'   -- 'pending', 'success', 'error'
);

-- Derived billing data
CREATE TABLE usage_events (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    api_key_id UUID,
    product VARCHAR(50) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    tokens INTEGER DEFAULT 0,
    requests INTEGER DEFAULT 1,
    units INTEGER DEFAULT 0,
    timestamp TIMESTAMP NOT NULL
);
```

## Usage Tracking API

### record_request()

Called **immediately** when a request arrives (before processing):

```rust
buffer.record_request(
    request_id,              // UUID for tracking
    organization_id,         // From token claims
    api_key_id,             // From token claims
    "embeddings".to_string(), // Product name
    "/v1/embed".to_string(), // Endpoint
    req.text.clone(),        // IMPORTANT: The original input text
    Some(serde_json::json!({ // Optional metadata
        "normalize": req.normalize
    })),
);
```

**Behavior:**
- Non-blocking (spawns async task)
- Immediate INSERT to `api_request_log`
- Creates audit trail of ALL requests (even failures)

### record_response()

Called **after processing** when response is ready:

```rust
buffer.record_response(
    request_id,          // Same ID from record_request
    organization_id,     // From token claims
    api_key_id,          // From token claims
    "embeddings",        // Product name
    token_count as i32,  // Calculated tokens
    serde_json::json!({  // Response metadata
        "model": "all-MiniLM-L6-v2",
        "cached": false,
        "latency_ms": 45.2
    }),
);
```

**Behavior:**
- Buffered (batched every 5 seconds)
- UPDATE `api_request_log` with response data
- INSERT to `usage_events` for billing
- Only called for successful requests

## Recalculation

### Scenario: Token counting bug discovered

```sql
-- 1. Check current token counts
SELECT
    DATE(request_timestamp) as date,
    COUNT(*) as requests,
    SUM(tokens) as total_tokens,
    AVG(tokens) as avg_tokens
FROM api_request_log
WHERE product = 'embeddings'
  AND status = 'success'
GROUP BY DATE(request_timestamp)
ORDER BY date DESC
LIMIT 7;

-- 2. Recalculate tokens with NEW algorithm
-- (This is a simplified example - you'd use a proper tokenizer)
UPDATE api_request_log
SET tokens = LENGTH(input_text) / 4  -- New algorithm
WHERE product = 'embeddings'
  AND status = 'success'
  AND request_timestamp >= '2025-01-01';

-- 3. Rebuild usage_events from corrected data
SELECT recalculate_usage('2025-01-01', '2025-02-01');

-- 4. Verify
SELECT
    DATE(timestamp) as date,
    SUM(tokens) as total_tokens
FROM usage_events
WHERE product = 'embeddings'
GROUP BY DATE(timestamp)
ORDER BY date DESC;
```

### Scenario: Pricing model changes

```sql
-- Recalculate all usage for a specific organization
DELETE FROM usage_events WHERE organization_id = '...';

INSERT INTO usage_events (organization_id, api_key_id, product, event_type, tokens, requests, timestamp)
SELECT
    organization_id,
    api_key_id,
    product,
    'inference' as event_type,
    tokens * 2 as tokens,  -- New pricing: 2x tokens
    1 as requests,
    response_timestamp
FROM api_request_log
WHERE organization_id = '...'
  AND status = 'success';
```

## Analytics Queries

### Failed request rate

```sql
SELECT
    DATE(request_timestamp) as date,
    COUNT(*) as total_requests,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as successful,
    SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
    ROUND(100.0 * SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) / COUNT(*), 2) as success_rate
FROM api_request_log
WHERE request_timestamp >= NOW() - INTERVAL '7 days'
GROUP BY DATE(request_timestamp)
ORDER BY date DESC;
```

### Token distribution

```sql
SELECT
    product,
    MIN(tokens) as min_tokens,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY tokens) as median_tokens,
    AVG(tokens)::INTEGER as avg_tokens,
    MAX(tokens) as max_tokens
FROM api_request_log
WHERE status = 'success'
  AND request_timestamp >= NOW() - INTERVAL '30 days'
GROUP BY product;
```

### Cached vs uncached performance

```sql
SELECT
    response_metadata->>'cached' as cached,
    COUNT(*) as requests,
    AVG((response_metadata->>'latency_ms')::FLOAT) as avg_latency_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY (response_metadata->>'latency_ms')::FLOAT) as p95_latency
FROM api_request_log
WHERE status = 'success'
  AND product = 'embeddings'
  AND request_timestamp >= NOW() - INTERVAL '24 hours'
GROUP BY response_metadata->>'cached';
```

## Data Retention & Archival

### Partition by month for efficient archival

```sql
-- Create monthly partitions
CREATE TABLE api_request_log_2025_01 PARTITION OF api_request_log
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

CREATE TABLE api_request_log_2025_02 PARTITION OF api_request_log
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
```

### Archive old data

```sql
-- Archive requests older than 90 days to cold storage
CREATE TABLE api_request_log_archive_2024 AS
SELECT * FROM api_request_log
WHERE request_timestamp < '2025-01-01';

-- Keep usage_events for billing
-- Delete old request logs
DELETE FROM api_request_log
WHERE request_timestamp < '2025-01-01';
```

## Adding New Products

### Example: Reranking endpoint

```rust
// 1. Record request
buffer.record_request(
    request_id,
    org_id,
    api_key_id,
    "rerank".to_string(),
    "/v1/rerank".to_string(),
    serde_json::to_string(&req.documents)?, // Store documents for recalc
    Some(serde_json::json!({
        "top_k": req.top_k,
        "query": req.query
    })),
);

// 2. Process reranking...

// 3. Record response
buffer.record_response(
    request_id,
    org_id,
    api_key_id,
    "rerank",
    0, // No tokens for reranking
    serde_json::json!({
        "model": "cross-encoder",
        "documents_processed": req.documents.len(),
        "top_k": req.top_k,
        "latency_ms": latency
    }),
);

// Usage event will have:
// - tokens: 0
// - requests: 1
// - units: documents.len() (for per-document billing)
```

## Best Practices

1. **Always store original input** - You can't recalculate without it
2. **Use JSONB for flexibility** - Product-specific metadata evolves
3. **Archive old data** - Keep raw logs for 90 days, then aggregate
4. **Monitor pending requests** - Track requests without responses (failures)
5. **Test recalculation** - Periodically verify you can rebuild usage_events
6. **Add product metadata** - Store enough context to understand the request later

## Performance Considerations

- **record_request()**: Non-blocking, ~1ms overhead
- **record_response()**: In-memory buffering, negligible overhead
- **Flush interval**: 5 seconds (configurable)
- **Storage**: ~500 bytes per request (depends on input_text length)
- **Indexes**: Optimized for time-range and organization queries

## Migration from Old System

```sql
-- Migrate existing usage table (if you have old data)
INSERT INTO usage_events (organization_id, api_key_id, product, event_type, tokens, requests, timestamp)
SELECT
    organization_id,
    api_key_id,
    'embeddings' as product,
    'inference' as event_type,
    tokens,
    embeddings_count as requests,
    timestamp
FROM usage_old;
```

## Recalculation Function

```sql
-- Recalculate usage for a date range
SELECT recalculate_usage('2025-01-01', '2025-02-01');

-- Recalculate all usage
SELECT recalculate_usage();

-- Returns: (deleted_count, inserted_count)
```
