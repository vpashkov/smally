-- Event sourcing architecture for recalculable usage
-- Two tables: api_request_log (source of truth) + usage_events (derived billing data)

-- Drop old usage table
DROP TABLE IF EXISTS usage;

-- 1. Immutable request/response log (source of truth for recalculation)
CREATE TABLE api_request_log (
    request_id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    api_key_id UUID NOT NULL REFERENCES api_keys(key_id) ON DELETE SET NULL,

    -- Product and endpoint
    product VARCHAR(50) NOT NULL,
    endpoint VARCHAR(100) NOT NULL,

    -- Request data (filled immediately when request arrives)
    input_text TEXT NOT NULL,
    input_metadata JSONB,
    request_timestamp TIMESTAMP NOT NULL,

    -- Response data (filled when response is ready, NULL if request failed)
    tokens INTEGER,
    response_metadata JSONB,
    response_timestamp TIMESTAMP,

    -- Status tracking
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'success', 'error'

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_api_request_log_org_time ON api_request_log(organization_id, request_timestamp DESC);
CREATE INDEX idx_api_request_log_product ON api_request_log(product);
CREATE INDEX idx_api_request_log_status ON api_request_log(status);
CREATE INDEX idx_api_request_log_timestamps ON api_request_log(request_timestamp, response_timestamp);

-- 2. Aggregated usage events (derived from api_request_log, can be recalculated)
CREATE TABLE usage_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    api_key_id UUID REFERENCES api_keys(key_id) ON DELETE SET NULL,

    -- Product identification
    product VARCHAR(50) NOT NULL,
    event_type VARCHAR(50) NOT NULL,

    -- Billing metrics
    tokens INTEGER DEFAULT 0,
    requests INTEGER DEFAULT 1,
    units INTEGER DEFAULT 0,

    -- Timestamp
    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes for billing queries
CREATE INDEX idx_usage_events_org_product ON usage_events(organization_id, product, timestamp);
CREATE INDEX idx_usage_events_product ON usage_events(product);
CREATE INDEX idx_usage_events_timestamp ON usage_events(timestamp);

-- Function to recalculate usage from request log
CREATE OR REPLACE FUNCTION recalculate_usage(
    p_start_date TIMESTAMP DEFAULT NULL,
    p_end_date TIMESTAMP DEFAULT NULL
) RETURNS TABLE (
    deleted_count BIGINT,
    inserted_count BIGINT
) AS $$
DECLARE
    v_start TIMESTAMP := COALESCE(p_start_date, '-infinity');
    v_end TIMESTAMP := COALESCE(p_end_date, 'infinity');
    v_deleted BIGINT;
    v_inserted BIGINT;
BEGIN
    -- Delete old calculated usage for the period
    DELETE FROM usage_events
    WHERE timestamp >= v_start AND timestamp < v_end;

    GET DIAGNOSTICS v_deleted = ROW_COUNT;

    -- Recalculate from successful requests in api_request_log
    INSERT INTO usage_events (organization_id, api_key_id, product, event_type, tokens, requests, timestamp)
    SELECT
        organization_id,
        api_key_id,
        product,
        'inference' as event_type,
        tokens,
        1 as requests,
        response_timestamp as timestamp
    FROM api_request_log
    WHERE response_timestamp >= v_start
      AND response_timestamp < v_end
      AND status = 'success'
      AND tokens IS NOT NULL;

    GET DIAGNOSTICS v_inserted = ROW_COUNT;

    RAISE NOTICE 'Recalculated usage: deleted %, inserted % rows', v_deleted, v_inserted;

    RETURN QUERY SELECT v_deleted, v_inserted;
END;
$$ LANGUAGE plpgsql;

-- Comments
COMMENT ON TABLE api_request_log IS 'Immutable request/response log - source of truth for usage recalculation';
COMMENT ON TABLE usage_events IS 'Aggregated usage events - derived from api_request_log, used for billing';
COMMENT ON COLUMN api_request_log.input_text IS 'Original input text - needed for recalculating tokens if algorithm changes';
COMMENT ON COLUMN api_request_log.tokens IS 'Token count - can be recalculated from input_text';
