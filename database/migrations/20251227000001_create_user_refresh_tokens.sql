-- User Refresh Tokens for JWT Authentication
--
-- Stores refresh tokens for user authentication.
-- Refresh tokens are long-lived and can be exchanged for new access tokens.
--
-- Security features:
-- - Tokens are hashed (SHA-256) before storage for fast lookup
-- - 256 bits of entropy makes brute-force infeasible
-- - Tokens can be revoked individually or all at once per user
-- - Automatic expiration after 30 days
-- - Token rotation: old token invalidated when new one is issued

CREATE TABLE IF NOT EXISTS user_refresh_tokens (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Token hash (SHA-256 hex) - for fast lookup
    -- Using SHA-256 is sufficient because tokens have 256 bits of entropy
    token_hash TEXT NOT NULL UNIQUE,

    -- Token metadata
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Revocation support
    revoked_at TIMESTAMPTZ,

    -- Device/session info (optional, for audit)
    user_agent TEXT,
    ip_address TEXT
);

-- Index for user's tokens (for revocation)
CREATE INDEX idx_user_refresh_tokens_user_id ON user_refresh_tokens (user_id);

-- Index for cleanup of expired tokens
CREATE INDEX idx_user_refresh_tokens_expires_at ON user_refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;

-- Comment
COMMENT ON TABLE user_refresh_tokens IS 'Refresh tokens for user JWT authentication';
