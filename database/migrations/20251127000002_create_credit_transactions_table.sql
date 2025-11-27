-- Migration: Create Credit Transactions Table
-- Description: Audit log for all credit balance changes
-- Created: 2025-11-27

-- Table: credit_transactions
-- Purpose: Immutable audit log of all credit operations
CREATE TABLE credit_transactions (
    id BIGSERIAL PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL,  -- Positive=add, negative=deduct
    transaction_type TEXT NOT NULL CHECK (transaction_type IN (
        'purchase',    -- Credit purchased via Stripe or x402
        'usage',       -- Credit used for API queries
        'refund',      -- Credit refunded
        'bonus',       -- Promotional credit added
        'adjustment'   -- Manual adjustment by admin
    )),
    description TEXT,
    reference_id TEXT,        -- External reference (Stripe payment ID, query ID, etc.)
    balance_after BIGINT NOT NULL,
    metadata JSONB,           -- Additional structured data
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common query patterns
CREATE INDEX idx_credit_transactions_org_created ON credit_transactions(organization_id, created_at DESC);
CREATE INDEX idx_credit_transactions_type_created ON credit_transactions(transaction_type, created_at DESC);
CREATE INDEX idx_credit_transactions_reference ON credit_transactions(reference_id) WHERE reference_id IS NOT NULL;

-- Comment on table
COMMENT ON TABLE credit_transactions IS 'Audit log of all credit balance changes';
COMMENT ON COLUMN credit_transactions.amount IS 'Credit amount change - positive for additions, negative for deductions';
COMMENT ON COLUMN credit_transactions.balance_after IS 'Credit balance after this transaction';
