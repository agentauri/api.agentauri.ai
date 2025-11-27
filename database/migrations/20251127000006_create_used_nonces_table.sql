-- Migration: Create Used Nonces Table
-- Description: Anti-replay protection for wallet authentication
-- Created: 2025-11-27

-- Table: used_nonces
-- Purpose: Track used nonces for wallet authentication to prevent replay attacks
CREATE TABLE used_nonces (
    nonce TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    used_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL     -- For cleanup job
);

-- Index for cleanup job (delete expired nonces)
CREATE INDEX idx_used_nonces_expires ON used_nonces(expires_at);
CREATE INDEX idx_used_nonces_wallet ON used_nonces(wallet_address);

-- Comment on table
COMMENT ON TABLE used_nonces IS 'Tracks used nonces to prevent replay attacks in wallet authentication';
COMMENT ON COLUMN used_nonces.expires_at IS 'Nonce expiration time - cleanup job deletes expired entries';

-- Note: Run cleanup periodically:
-- DELETE FROM used_nonces WHERE expires_at < NOW() - INTERVAL '24 hours'
