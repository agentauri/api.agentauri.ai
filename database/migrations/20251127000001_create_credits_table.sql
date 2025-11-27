-- Migration: Create Credits Table
-- Description: Organization credit balance for payment system
-- Created: 2025-11-27

-- Table: credits
-- Purpose: Store credit balance per organization (1:1 relationship)
-- Credits are stored in micro-USDC (6 decimals) for precision
CREATE TABLE credits (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    balance BIGINT NOT NULL DEFAULT 0,  -- In micro-USDC (6 decimals)
    currency TEXT NOT NULL DEFAULT 'USDC',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT credits_organization_unique UNIQUE (organization_id),
    CONSTRAINT credits_balance_non_negative CHECK (balance >= 0)
);

-- Index for fast organization lookups
CREATE INDEX idx_credits_organization_id ON credits(organization_id);

-- Trigger for automatic updated_at maintenance
CREATE TRIGGER update_credits_updated_at
    BEFORE UPDATE ON credits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Comment on table
COMMENT ON TABLE credits IS 'Organization credit balances for the payment system';
COMMENT ON COLUMN credits.balance IS 'Credit balance in micro-USDC (6 decimals, 1 USDC = 1000000)';
