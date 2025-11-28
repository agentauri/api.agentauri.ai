-- Migration: Create oauth_tokens table
-- Description: OAuth 2.0 access and refresh tokens for authorized applications
-- Phase: 4 Week 13 - OAuth 2.0 Infrastructure
-- Security: Tokens stored as SHA-256 hashes for secure lookups

CREATE TABLE oauth_tokens (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    access_token_hash TEXT UNIQUE NOT NULL,       -- SHA-256 hash of access token
    refresh_token_hash TEXT UNIQUE,               -- SHA-256 hash of refresh token (nullable)
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,                -- Scoped to organization for resource access
    scopes TEXT[] NOT NULL,                       -- Granted scopes for this token
    expires_at TIMESTAMPTZ NOT NULL,              -- Access token expiration
    refresh_token_expires_at TIMESTAMPTZ,         -- Refresh token expiration (nullable)
    revoked BOOLEAN NOT NULL DEFAULT false,       -- Revoked tokens are invalid
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_client FOREIGN KEY (client_id)
        REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    CONSTRAINT fk_user FOREIGN KEY (user_id)
        REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_organization FOREIGN KEY (organization_id)
        REFERENCES organizations(id) ON DELETE CASCADE,
    CONSTRAINT check_scopes_not_empty CHECK (array_length(scopes, 1) > 0),
    CONSTRAINT check_refresh_token_consistency
        CHECK ((refresh_token_hash IS NULL AND refresh_token_expires_at IS NULL) OR
               (refresh_token_hash IS NOT NULL AND refresh_token_expires_at IS NOT NULL))
);

-- Partial index for active access token lookups (hot path for API authentication)
CREATE INDEX idx_oauth_tokens_access_token ON oauth_tokens(access_token_hash)
    WHERE NOT revoked;

-- Partial index for active refresh token lookups (token refresh flow)
CREATE INDEX idx_oauth_tokens_refresh_token ON oauth_tokens(refresh_token_hash)
    WHERE NOT revoked AND refresh_token_hash IS NOT NULL;

-- Index for listing user's active tokens (user token management)
CREATE INDEX idx_oauth_tokens_user ON oauth_tokens(user_id, created_at DESC);

-- Index for listing client's active tokens (client monitoring)
CREATE INDEX idx_oauth_tokens_client ON oauth_tokens(client_id, created_at DESC);

-- Partial index for cleanup jobs (finding expired tokens)
CREATE INDEX idx_oauth_tokens_expires ON oauth_tokens(expires_at)
    WHERE NOT revoked;

-- Index for organization-scoped token lookups
CREATE INDEX idx_oauth_tokens_organization ON oauth_tokens(organization_id);

COMMENT ON TABLE oauth_tokens IS 'OAuth 2.0 access and refresh tokens for authorized third-party applications';
COMMENT ON COLUMN oauth_tokens.access_token_hash IS 'SHA-256 hash of access token for secure lookup';
COMMENT ON COLUMN oauth_tokens.refresh_token_hash IS 'SHA-256 hash of refresh token (null for client_credentials grant)';
COMMENT ON COLUMN oauth_tokens.scopes IS 'Array of scopes granted to this token (subset of client allowed scopes)';
COMMENT ON COLUMN oauth_tokens.expires_at IS 'Access token expiration (typically 1 hour)';
COMMENT ON COLUMN oauth_tokens.refresh_token_expires_at IS 'Refresh token expiration (typically 30 days)';
COMMENT ON COLUMN oauth_tokens.revoked IS 'When true, token is permanently invalid (logout, security incident)';
COMMENT ON COLUMN oauth_tokens.organization_id IS 'Organization context for resource access control';
