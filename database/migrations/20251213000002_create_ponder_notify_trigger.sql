-- ============================================================================
-- Migration: Create NOTIFY Trigger on Ponder Event Table
-- ============================================================================
-- IMPORTANT: This migration is conditional - it only runs if Ponder has created
-- its Event table in the ponder schema. Safe to run before or after Ponder.
--
-- This trigger enables real-time event processing by sending PostgreSQL NOTIFY
-- whenever Ponder inserts a new blockchain event.
--
-- Created: 2025-12-13
-- Updated: 2025-12-25 (made conditional for local dev without Ponder)
-- ============================================================================

-- Conditional trigger creation - only if ponder."Event" exists
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'ponder' AND table_name = 'Event'
    ) THEN
        -- Drop existing trigger if present (for re-running migration)
        DROP TRIGGER IF EXISTS trigger_notify_ponder_event ON ponder."Event";

        -- Create trigger on Ponder's Event table
        CREATE TRIGGER trigger_notify_ponder_event
            AFTER INSERT ON ponder."Event"
            FOR EACH ROW
            EXECUTE FUNCTION ponder.notify_ponder_event();

        -- Add comment for documentation
        COMMENT ON TRIGGER trigger_notify_ponder_event ON ponder."Event" IS
        'Sends PostgreSQL NOTIFY on new_event channel when Ponder indexes blockchain events';

        -- Create indexes for faster event retrieval
        CREATE INDEX IF NOT EXISTS idx_ponder_event_id_chain_id
            ON ponder."Event"(id, "chainId");

        CREATE INDEX IF NOT EXISTS idx_ponder_event_registry_chain
            ON ponder."Event"(registry, "chainId");

        CREATE INDEX IF NOT EXISTS idx_ponder_event_timestamp
            ON ponder."Event"("timestamp");

        RAISE NOTICE 'Ponder trigger and indexes created successfully';
    ELSE
        RAISE NOTICE 'Skipping Ponder trigger creation - ponder."Event" table does not exist yet';
    END IF;
END $$;
