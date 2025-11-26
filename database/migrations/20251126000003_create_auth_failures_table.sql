-- Migration: Create auth_failures table
-- Description: Track authentication failures without organization context
-- Phase: 3.5b - API Key Security Hardening
-- Security: Essential for detecting brute-force attacks and security analysis

-- This table captures auth failures where we cannot determine the organization,
-- such as when an invalid key prefix is provided. This is critical for:
-- 1. Detecting brute-force attacks by IP
-- 2. Identifying potential key enumeration attempts
-- 3. Rate limit enforcement at the IP level

CREATE TABLE auth_failures (
    id BIGSERIAL PRIMARY KEY,
    failure_type TEXT NOT NULL CHECK (failure_type IN (
        'invalid_format',     -- Key format invalid (not sk_live/sk_test prefix)
        'prefix_not_found',   -- Prefix exists but no matching key in DB
        'rate_limited',       -- Request blocked by rate limiter
        'invalid_key'         -- Key found but hash verification failed (shouldn't happen with known org)
    )),
    key_prefix TEXT,          -- First 16 chars if available (for pattern analysis)
    ip_address TEXT,
    user_agent TEXT,
    endpoint TEXT,
    details JSONB,            -- Additional context
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for rate limit enforcement and brute-force detection
CREATE INDEX idx_auth_failures_ip ON auth_failures(ip_address, created_at DESC);

-- Index for pattern analysis (seeing which prefixes are being tried)
CREATE INDEX idx_auth_failures_prefix ON auth_failures(key_prefix, created_at DESC)
    WHERE key_prefix IS NOT NULL;

-- Index for time-based analysis
CREATE INDEX idx_auth_failures_time ON auth_failures(created_at DESC);

-- Retention: Auto-delete old records (recommended: keep 30 days)
-- Note: Uncomment for TimescaleDB environments
-- SELECT create_hypertable('auth_failures', 'created_at', if_not_exists => TRUE);

COMMENT ON TABLE auth_failures IS 'Authentication failures without organization context for security analysis';
COMMENT ON COLUMN auth_failures.failure_type IS 'Type of failure: invalid_format, prefix_not_found, rate_limited, invalid_key';
COMMENT ON COLUMN auth_failures.key_prefix IS 'First 16 chars of attempted key (if available) for pattern detection';
