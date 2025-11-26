-- Migration: Create api_keys table
-- Description: API key storage for Layer 1 authentication with Argon2id hashing
-- Phase: 3.5 - API Key Authentication (Layer 1)
-- Security: Keys are stored as Argon2id hashes, never plaintext

CREATE TABLE api_keys (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,                   -- Argon2id hash of the full key
    name TEXT NOT NULL,
    prefix TEXT NOT NULL UNIQUE,              -- sk_live_XXXXXXXX or sk_test_XXXXXXXX (first 16 chars)
    environment TEXT NOT NULL CHECK (environment IN ('live', 'test')),
    key_type TEXT NOT NULL DEFAULT 'standard'
        CHECK (key_type IN ('standard', 'restricted', 'admin')),
    permissions JSONB NOT NULL DEFAULT '["read"]'::JSONB,
    rate_limit_override INTEGER,              -- NULL means use plan default
    last_used_at TIMESTAMPTZ,
    last_used_ip TEXT,
    expires_at TIMESTAMPTZ,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    revoked_by TEXT REFERENCES users(id),
    revocation_reason TEXT
);

-- Index for lookup during authentication (active keys only)
-- This is the hot path - must be fast
CREATE INDEX idx_api_keys_prefix ON api_keys(prefix) WHERE revoked_at IS NULL;

-- Index for listing organization's keys (active only)
CREATE INDEX idx_api_keys_organization ON api_keys(organization_id) WHERE revoked_at IS NULL;

-- Index for audit queries (include revoked keys)
CREATE INDEX idx_api_keys_organization_all ON api_keys(organization_id, created_at DESC);

-- Index for finding expired keys (cleanup job)
CREATE INDEX idx_api_keys_expires ON api_keys(expires_at) WHERE expires_at IS NOT NULL AND revoked_at IS NULL;

COMMENT ON TABLE api_keys IS 'API keys for Layer 1 authentication - keys stored as Argon2id hashes only';
COMMENT ON COLUMN api_keys.key_hash IS 'Argon2id hash of the full API key (64MiB memory, 3 iterations)';
COMMENT ON COLUMN api_keys.prefix IS 'First 16 characters of key (sk_live_XXXXXXXX) for lookup without exposing full key';
COMMENT ON COLUMN api_keys.environment IS 'live for production, test for development/testing';
COMMENT ON COLUMN api_keys.key_type IS 'standard: normal ops, restricted: limited permissions, admin: full access';
COMMENT ON COLUMN api_keys.permissions IS 'JSON array of permission strings: read, write, delete, admin';
COMMENT ON COLUMN api_keys.rate_limit_override IS 'Custom rate limit, NULL uses plan default';
COMMENT ON COLUMN api_keys.revoked_at IS 'When set, key is permanently invalid';
