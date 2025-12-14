-- ============================================================================
-- Migration: Create NOTIFY Trigger on Ponder Event Table
-- ============================================================================
-- IMPORTANT: This migration must be run AFTER Ponder has started and created
-- its Event table in the ponder schema. Run this manually after first Ponder
-- deployment, or use a post-deployment hook.
--
-- This trigger enables real-time event processing by sending PostgreSQL NOTIFY
-- whenever Ponder inserts a new blockchain event.
--
-- Created: 2025-12-13
-- ============================================================================

-- Drop existing trigger if present (for re-running migration)
DROP TRIGGER IF EXISTS trigger_notify_ponder_event ON ponder."Event";

-- Create trigger on Ponder's Event table
-- Note: Ponder creates tables with PascalCase names that require quoting
CREATE TRIGGER trigger_notify_ponder_event
    AFTER INSERT ON ponder."Event"
    FOR EACH ROW
    EXECUTE FUNCTION ponder.notify_ponder_event();

-- Add comment for documentation
COMMENT ON TRIGGER trigger_notify_ponder_event ON ponder."Event" IS
'Sends PostgreSQL NOTIFY on new_event channel when Ponder indexes blockchain events';

-- Create index for faster event retrieval by event-processor
-- Using the same pattern as the existing events table
CREATE INDEX IF NOT EXISTS idx_ponder_event_id_chain_id
    ON ponder."Event"(id, "chainId");

CREATE INDEX IF NOT EXISTS idx_ponder_event_registry_chain
    ON ponder."Event"(registry, "chainId");

CREATE INDEX IF NOT EXISTS idx_ponder_event_timestamp
    ON ponder."Event"("timestamp");
