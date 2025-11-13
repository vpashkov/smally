-- Performance optimization indexes
-- Run this script to add indexes for faster API key lookups and rate limiting

-- Index for fast API key lookups (on cache miss)
-- This index helps the JOIN query in ApiKeyCache.validate()
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_api_keys_hash_active
ON api_keys(key_hash)
WHERE is_active = true;

-- Index for fast rate limit queries (on Redis fallback)
-- This index helps the SUM query in check_rate_limit_db()
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_usage_rate_limit
ON usage(user_id, api_key_id, timestamp);

-- Optional: Index for faster user lookups (less critical with cache)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_users_active
ON users(id)
WHERE is_active = true;

-- Analyze tables to update statistics
ANALYZE api_keys;
ANALYZE usage;
ANALYZE users;
