-- Migration: Create ponder_provider_reputation table
-- Tracks RPC provider performance and reputation across restarts

CREATE TABLE IF NOT EXISTS ponder_provider_reputation (
    id SERIAL PRIMARY KEY,

    -- Provider identification
    chain_id INTEGER NOT NULL,
    chain_name VARCHAR(50) NOT NULL,
    provider_name VARCHAR(50) NOT NULL,

    -- Request metrics
    total_requests BIGINT NOT NULL DEFAULT 0,
    successful_requests BIGINT NOT NULL DEFAULT 0,
    failed_requests BIGINT NOT NULL DEFAULT 0,

    -- Latency metrics (in milliseconds)
    avg_latency_ms DOUBLE PRECISION,
    min_latency_ms DOUBLE PRECISION,
    max_latency_ms DOUBLE PRECISION,
    p50_latency_ms DOUBLE PRECISION,
    p95_latency_ms DOUBLE PRECISION,
    p99_latency_ms DOUBLE PRECISION,

    -- Circuit breaker state
    circuit_state VARCHAR(20) NOT NULL DEFAULT 'closed',
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_failure_at TIMESTAMPTZ,
    last_success_at TIMESTAMPTZ,

    -- Quota tracking
    daily_requests INTEGER NOT NULL DEFAULT 0,
    monthly_requests INTEGER NOT NULL DEFAULT 0,
    daily_quota_limit INTEGER,
    monthly_quota_limit INTEGER,
    last_daily_reset TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_monthly_reset TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Rate limiting
    rate_limited_until TIMESTAMPTZ,
    rate_limit_count INTEGER NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint per chain + provider
    CONSTRAINT uq_ponder_provider_reputation_chain_provider
        UNIQUE (chain_id, provider_name)
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_ponder_provider_reputation_chain_id
    ON ponder_provider_reputation(chain_id);

CREATE INDEX IF NOT EXISTS idx_ponder_provider_reputation_provider_name
    ON ponder_provider_reputation(provider_name);

CREATE INDEX IF NOT EXISTS idx_ponder_provider_reputation_circuit_state
    ON ponder_provider_reputation(circuit_state);

CREATE INDEX IF NOT EXISTS idx_ponder_provider_reputation_updated_at
    ON ponder_provider_reputation(updated_at);

-- Trigger to update updated_at on changes
CREATE OR REPLACE FUNCTION update_ponder_provider_reputation_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_ponder_provider_reputation_updated_at
    ON ponder_provider_reputation;

CREATE TRIGGER trigger_ponder_provider_reputation_updated_at
    BEFORE UPDATE ON ponder_provider_reputation
    FOR EACH ROW
    EXECUTE FUNCTION update_ponder_provider_reputation_updated_at();

-- Add helpful comments
COMMENT ON TABLE ponder_provider_reputation IS 'Tracks RPC provider performance metrics and circuit breaker state for Ponder indexer resilience';
COMMENT ON COLUMN ponder_provider_reputation.circuit_state IS 'Circuit breaker state: closed, open, or half-open';
COMMENT ON COLUMN ponder_provider_reputation.consecutive_failures IS 'Number of consecutive failures before circuit opened';
