-- Migration: Create user_identities table for multi-provider authentication
-- This table enables users to authenticate via multiple providers (email, Google, GitHub, wallet)
-- and link them to a single account.

-- ============================================================================
-- Create user_identities table
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_identities (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Provider identification
    provider TEXT NOT NULL,              -- 'email', 'google', 'github', 'wallet'
    provider_user_id TEXT NOT NULL,      -- Unique ID from the provider

    -- Profile data from provider
    email TEXT,                          -- Email from provider (if available)
    display_name TEXT,                   -- Display name from provider
    avatar_url TEXT,                     -- Avatar URL from provider

    -- Wallet-specific fields (only for provider='wallet')
    wallet_address TEXT,                 -- Ethereum address (checksummed)
    chain_id INTEGER,                    -- Chain ID (e.g., 1 for mainnet, 11155111 for sepolia)

    -- OAuth tokens (encrypted at rest)
    access_token_encrypted TEXT,         -- Encrypted OAuth access token
    refresh_token_encrypted TEXT,        -- Encrypted OAuth refresh token
    token_expires_at TIMESTAMPTZ,        -- Token expiration time

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,

    -- Ensure unique provider + provider_user_id combination
    CONSTRAINT user_identities_provider_unique UNIQUE (provider, provider_user_id)
);

-- Index for fast user_id lookups (list all identities for a user)
CREATE INDEX IF NOT EXISTS idx_user_identities_user_id
    ON user_identities(user_id);

-- Index for fast provider lookups (find identity by provider + provider_user_id)
CREATE INDEX IF NOT EXISTS idx_user_identities_provider_lookup
    ON user_identities(provider, provider_user_id);

-- Index for email lookups (find identity by email across providers)
CREATE INDEX IF NOT EXISTS idx_user_identities_email
    ON user_identities(email)
    WHERE email IS NOT NULL;

-- Partial unique index for wallet addresses (only when provider='wallet')
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_identities_wallet_unique
    ON user_identities(wallet_address, chain_id)
    WHERE provider = 'wallet' AND wallet_address IS NOT NULL;

-- ============================================================================
-- Modify users table for social auth support
-- ============================================================================

-- Make password_hash optional (users with only social login won't have a password)
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- Add primary auth provider column
ALTER TABLE users ADD COLUMN IF NOT EXISTS primary_auth_provider TEXT DEFAULT 'email';

-- Add avatar_url column for profile pictures
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url TEXT;

-- ============================================================================
-- Migrate existing users to user_identities (email provider)
-- ============================================================================

-- Create email identity for existing users with password
INSERT INTO user_identities (user_id, provider, provider_user_id, email, created_at)
SELECT
    id,
    'email',
    email,  -- Use email as provider_user_id for email provider
    email,
    created_at
FROM users
WHERE password_hash IS NOT NULL
ON CONFLICT (provider, provider_user_id) DO NOTHING;

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE user_identities IS 'Links multiple authentication providers to a single user account';
COMMENT ON COLUMN user_identities.provider IS 'Authentication provider: email, google, github, wallet';
COMMENT ON COLUMN user_identities.provider_user_id IS 'Unique identifier from the provider (e.g., Google sub, GitHub id)';
COMMENT ON COLUMN user_identities.access_token_encrypted IS 'Encrypted OAuth access token for API calls to provider';
COMMENT ON COLUMN user_identities.wallet_address IS 'Checksummed Ethereum address for wallet authentication';
