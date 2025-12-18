-- Migration: Create payment_nonces table
-- Description: Stores payment nonces for x402 cryptocurrency payment integration
-- Created: 2025-12-17

CREATE TABLE IF NOT EXISTS payment_nonces (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    nonce TEXT NOT NULL UNIQUE,
    amount BIGINT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USDC',
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'expired', 'failed')),
    payment_method TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for finding nonces by organization
CREATE INDEX IF NOT EXISTS idx_payment_nonces_organization_id ON payment_nonces(organization_id);

-- Index for finding pending nonces (for cleanup job)
CREATE INDEX IF NOT EXISTS idx_payment_nonces_status_expires
    ON payment_nonces(status, expires_at)
    WHERE status = 'pending';

-- Index for unique nonce lookup
CREATE INDEX IF NOT EXISTS idx_payment_nonces_nonce ON payment_nonces(nonce);

COMMENT ON TABLE payment_nonces IS 'Payment nonces for x402 cryptocurrency payment integration';
COMMENT ON COLUMN payment_nonces.amount IS 'Payment amount in micro-USDC (6 decimals)';
COMMENT ON COLUMN payment_nonces.status IS 'Nonce status: pending, completed, expired, or failed';
COMMENT ON COLUMN payment_nonces.expires_at IS 'Expiration time for pending nonces';
