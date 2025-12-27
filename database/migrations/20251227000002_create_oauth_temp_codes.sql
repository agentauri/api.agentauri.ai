-- OAuth Temporary Authorization Codes
-- These are short-lived codes (5 min) used to securely exchange for tokens
-- This prevents token exposure in URLs/browser history (Authorization Code Flow)

CREATE TABLE IF NOT EXISTS oauth_temp_codes (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    code_hash TEXT NOT NULL UNIQUE,  -- SHA-256 hash of the code
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,  -- Set when code is exchanged for tokens
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for fast lookup by code hash
CREATE INDEX IF NOT EXISTS idx_oauth_temp_codes_code_hash ON oauth_temp_codes (code_hash);

-- Index for cleanup of expired unused codes
CREATE INDEX IF NOT EXISTS idx_oauth_temp_codes_expires ON oauth_temp_codes (expires_at) WHERE used_at IS NULL;

-- Index for user lookup (e.g., to invalidate pending codes)
CREATE INDEX IF NOT EXISTS idx_oauth_temp_codes_user_id ON oauth_temp_codes (user_id);
