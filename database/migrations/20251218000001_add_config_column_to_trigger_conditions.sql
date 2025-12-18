-- Migration: Add config column to trigger_conditions
-- Description: Adds optional JSONB config column for advanced condition configuration
-- Created: 2025-12-18
-- Applied manually to production: 2025-12-18

-- Add config column for advanced condition configuration (e.g., window_size for EMA)
ALTER TABLE trigger_conditions ADD COLUMN IF NOT EXISTS config JSONB;

COMMENT ON COLUMN trigger_conditions.config IS 'Optional JSONB config for advanced condition settings (e.g., window_size for EMA evaluator)';
