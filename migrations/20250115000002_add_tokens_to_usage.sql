-- Add tokens column to usage table for token-based billing
-- Keep embeddings_count for tracking number of requests

ALTER TABLE usage ADD COLUMN tokens INTEGER NOT NULL DEFAULT 0;

-- Add index for efficient token usage queries
CREATE INDEX idx_usage_tokens ON usage(tokens);

-- Update comment
COMMENT ON COLUMN usage.embeddings_count IS 'Number of embedding requests (for request-based analytics)';
COMMENT ON COLUMN usage.tokens IS 'Number of tokens processed (for token-based billing)';
