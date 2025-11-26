-- Migration: Create api_key_audit_log table
-- Description: Audit trail for all API key operations and authentication events
-- Phase: 3.5 - API Key Authentication (Layer 1)
-- Security: Immutable log for security analysis and compliance

CREATE TABLE api_key_audit_log (
    id BIGSERIAL PRIMARY KEY,
    api_key_id TEXT,                          -- NULL for auth failures (key not found)
    organization_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'created',        -- Key was created
        'used',           -- Key was used for authentication (success)
        'rotated',        -- Key was rotated (new key generated)
        'revoked',        -- Key was revoked
        'auth_failed',    -- Authentication attempt failed
        'rate_limited'    -- Request was rate limited
    )),
    ip_address TEXT,
    user_agent TEXT,
    endpoint TEXT,                            -- API endpoint accessed
    actor_user_id TEXT,                       -- User who performed the action (for mgmt ops)
    details JSONB,                            -- Additional context (error reason, etc.)
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for viewing key's audit history
CREATE INDEX idx_api_key_audit_key ON api_key_audit_log(api_key_id, created_at DESC);

-- Index for organization-wide audit queries
CREATE INDEX idx_api_key_audit_org ON api_key_audit_log(organization_id, created_at DESC);

-- Index for security analysis: find auth failures by IP
CREATE INDEX idx_api_key_audit_failures ON api_key_audit_log(ip_address, created_at DESC)
    WHERE event_type = 'auth_failed';

-- Index for rate limit analysis by IP
CREATE INDEX idx_api_key_audit_rate_limited ON api_key_audit_log(ip_address, created_at DESC)
    WHERE event_type = 'rate_limited';

-- Index for event type analysis
CREATE INDEX idx_api_key_audit_event_type ON api_key_audit_log(event_type, created_at DESC);

COMMENT ON TABLE api_key_audit_log IS 'Immutable audit trail for API key operations and authentication events';
COMMENT ON COLUMN api_key_audit_log.api_key_id IS 'Reference to api_key (NULL for failed lookups where key not found)';
COMMENT ON COLUMN api_key_audit_log.event_type IS 'Type of event: created, used, rotated, revoked, auth_failed, rate_limited';
COMMENT ON COLUMN api_key_audit_log.actor_user_id IS 'User who triggered the event (for management operations)';
COMMENT ON COLUMN api_key_audit_log.details IS 'Additional context: error reasons, old key ID for rotations, etc.';
