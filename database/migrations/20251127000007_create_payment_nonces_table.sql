-- Migration: Create Payment Nonces Table
-- Description: Idempotency keys for payment operations (x402 basics)
-- Created: 2025-11-27

-- Table: payment_nonces
-- Purpose: Track payment nonces for idempotency in payment operations
-- Enables future x402 crypto payment support
CREATE TABLE payment_nonces (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    nonce TEXT UNIQUE NOT NULL,         -- Idempotency key
    amount BIGINT NOT NULL,             -- Payment amount in micro-units
    currency TEXT NOT NULL DEFAULT 'USDC',
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending',     -- Awaiting payment
        'completed',   -- Payment successful
        'expired',     -- Nonce expired
        'failed'       -- Payment failed
    )),
    payment_method TEXT NOT NULL CHECK (payment_method IN (
        'stripe',      -- Stripe payment
        'x402',        -- x402 crypto payment (future)
        'credits'      -- Credit deduction
    )),
    expires_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_payment_nonces_organization ON payment_nonces(organization_id);
CREATE INDEX idx_payment_nonces_nonce ON payment_nonces(nonce);
CREATE INDEX idx_payment_nonces_pending ON payment_nonces(status) WHERE status = 'pending';
CREATE INDEX idx_payment_nonces_expires ON payment_nonces(expires_at) WHERE status = 'pending';

-- Comment on table
COMMENT ON TABLE payment_nonces IS 'Payment idempotency tracking for x402 and other payment methods';
COMMENT ON COLUMN payment_nonces.nonce IS 'Unique idempotency key for the payment operation';
COMMENT ON COLUMN payment_nonces.amount IS 'Payment amount in micro-units (6 decimals for USDC)';
