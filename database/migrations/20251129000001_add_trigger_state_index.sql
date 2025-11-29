-- Migration: Add index for trigger_state cleanup operations
-- Description: Adds index on last_updated column to optimize state cleanup queries
-- Created: 2025-11-29

-- Add index for cleanup operations
CREATE INDEX IF NOT EXISTS idx_trigger_state_last_updated ON trigger_state(last_updated);

-- Comment explaining the index
COMMENT ON INDEX idx_trigger_state_last_updated IS 'Optimizes cleanup of expired trigger state records';
