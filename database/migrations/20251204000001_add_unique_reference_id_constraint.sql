-- Migration: Add UNIQUE constraint on reference_id
-- Description: Prevents race condition in webhook idempotency check
-- Security: Fixes CVSS 7.1 Stripe webhook replay attack vulnerability
-- Created: 2025-12-04

-- Add unique partial index on reference_id for purchases only
-- This ensures atomic idempotency for Stripe webhooks
-- Only applies to non-NULL reference_ids (partial index)
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_credit_transactions_reference_unique
ON credit_transactions(reference_id)
WHERE reference_id IS NOT NULL AND transaction_type = 'purchase';

-- Drop the old non-unique index (now redundant)
DROP INDEX IF EXISTS idx_credit_transactions_reference;

COMMENT ON INDEX idx_credit_transactions_reference_unique IS 'Ensures idempotency for external payment references';
