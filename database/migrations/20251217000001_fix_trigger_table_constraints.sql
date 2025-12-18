-- Migration: Fix NULL constraints in trigger_conditions and trigger_actions tables
-- Description: Add NOT NULL constraints to columns that should never be NULL
-- This fixes schema mismatches with the Rust models that expect non-null values.
-- Created: 2025-12-17

-- First, update any NULL values to defaults (in case any exist)
UPDATE trigger_conditions
SET created_at = NOW()
WHERE created_at IS NULL;

UPDATE trigger_actions
SET priority = 1
WHERE priority IS NULL;

UPDATE trigger_actions
SET created_at = NOW()
WHERE created_at IS NULL;

-- Now add NOT NULL constraints

-- trigger_conditions.created_at: already has DEFAULT NOW(), add NOT NULL
ALTER TABLE trigger_conditions
ALTER COLUMN created_at SET NOT NULL;

-- trigger_actions.priority: already has DEFAULT 1, add NOT NULL
ALTER TABLE trigger_actions
ALTER COLUMN priority SET NOT NULL;

-- trigger_actions.created_at: already has DEFAULT NOW(), add NOT NULL
ALTER TABLE trigger_actions
ALTER COLUMN created_at SET NOT NULL;

-- Add comment for documentation
COMMENT ON COLUMN trigger_conditions.created_at IS 'Timestamp when the condition was created (NOT NULL, defaults to NOW())';
COMMENT ON COLUMN trigger_actions.priority IS 'Action priority (NOT NULL, defaults to 1, lower = higher priority)';
COMMENT ON COLUMN trigger_actions.created_at IS 'Timestamp when the action was created (NOT NULL, defaults to NOW())';
