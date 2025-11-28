-- Migration: Create oauth_clients table
-- Description: OAuth 2.0 client applications for third-party integrations
-- Phase: 4 Week 13 - OAuth 2.0 Infrastructure
-- Security: Client secrets stored as Argon2id hashes, never plaintext

CREATE TABLE oauth_clients (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    client_id TEXT UNIQUE NOT NULL,               -- Public client identifier
    client_secret_hash TEXT NOT NULL,             -- Argon2id hash of client secret
    client_name TEXT NOT NULL,                    -- Display name for the application
    redirect_uris TEXT[] NOT NULL,                -- Allowed OAuth redirect URIs
    scopes TEXT[] NOT NULL,                       -- Allowed scopes (e.g., ['read:triggers', 'write:triggers'])
    owner_organization_id TEXT NOT NULL,
    grant_types TEXT[] NOT NULL DEFAULT ARRAY['authorization_code', 'refresh_token'],
    is_trusted BOOLEAN NOT NULL DEFAULT false,    -- If true, skip consent screen
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_owner_organization FOREIGN KEY (owner_organization_id)
        REFERENCES organizations(id) ON DELETE CASCADE,
    CONSTRAINT check_redirect_uris_not_empty CHECK (array_length(redirect_uris, 1) > 0),
    CONSTRAINT check_scopes_not_empty CHECK (array_length(scopes, 1) > 0),
    CONSTRAINT check_grant_types_not_empty CHECK (array_length(grant_types, 1) > 0)
);

-- Index for listing organization's OAuth applications
CREATE INDEX idx_oauth_clients_organization ON oauth_clients(owner_organization_id);

-- Index for client authentication lookups (hot path)
CREATE INDEX idx_oauth_clients_client_id ON oauth_clients(client_id);

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER update_oauth_clients_updated_at
    BEFORE UPDATE ON oauth_clients
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE oauth_clients IS 'OAuth 2.0 client applications for third-party integrations';
COMMENT ON COLUMN oauth_clients.client_id IS 'Public OAuth client identifier (e.g., oauth_client_abc123)';
COMMENT ON COLUMN oauth_clients.client_secret_hash IS 'Argon2id hash of client secret (64MiB memory, 3 iterations)';
COMMENT ON COLUMN oauth_clients.redirect_uris IS 'Array of allowed redirect URIs for OAuth flow';
COMMENT ON COLUMN oauth_clients.scopes IS 'Array of scopes this client can request (e.g., read:triggers, write:triggers, read:billing)';
COMMENT ON COLUMN oauth_clients.grant_types IS 'Allowed OAuth grant types (authorization_code, refresh_token, client_credentials)';
COMMENT ON COLUMN oauth_clients.is_trusted IS 'First-party apps skip consent screen when true';
