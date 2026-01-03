-- ============================================================================
-- Migration: Add Automatic Trigger for Ponder Namespace Changes
-- ============================================================================
-- PROBLEM: When Ponder restarts, it creates a new namespace (e.g., 'ac57' -> 'bf12')
-- and updates _ponder_meta.live with the new instance_id. The ponder_events view
-- becomes stale and points to the old namespace tables.
--
-- SOLUTION: Create a trigger that automatically calls recreate_ponder_events_view()
-- whenever _ponder_meta.live is inserted or updated.
--
-- Created: 2026-01-02
-- ============================================================================

-- Trigger function that calls recreate_ponder_events_view()
CREATE OR REPLACE FUNCTION trigger_ponder_namespace_changed()
RETURNS TRIGGER AS $$
BEGIN
    -- Only act when the 'live' key is modified
    IF NEW.key = 'live' THEN
        -- Small delay to allow Ponder to finish creating tables
        -- (the view recreation needs the new tables to exist)
        PERFORM pg_sleep(2);

        -- Recreate views with new namespace
        PERFORM recreate_ponder_events_view();

        RAISE NOTICE 'Ponder namespace changed, views recreated automatically';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Drop existing trigger if it exists (idempotent)
DROP TRIGGER IF EXISTS ponder_namespace_changed ON _ponder_meta;

-- Create trigger on _ponder_meta table
-- Note: This will fail gracefully if _ponder_meta doesn't exist yet
-- (Ponder creates it on first run)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'public' AND tablename = '_ponder_meta') THEN
        CREATE TRIGGER ponder_namespace_changed
            AFTER INSERT OR UPDATE ON _ponder_meta
            FOR EACH ROW
            EXECUTE FUNCTION trigger_ponder_namespace_changed();

        RAISE NOTICE 'Trigger ponder_namespace_changed created successfully';
    ELSE
        RAISE NOTICE '_ponder_meta table does not exist yet. Trigger will be created when Ponder first runs.';

        -- Create a startup function that adds the trigger when _ponder_meta exists
        CREATE OR REPLACE FUNCTION ensure_ponder_namespace_trigger()
        RETURNS void AS $FUNC$
        BEGIN
            IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'public' AND tablename = '_ponder_meta') THEN
                -- Check if trigger already exists
                IF NOT EXISTS (
                    SELECT 1 FROM pg_trigger
                    WHERE tgname = 'ponder_namespace_changed'
                ) THEN
                    CREATE TRIGGER ponder_namespace_changed
                        AFTER INSERT OR UPDATE ON _ponder_meta
                        FOR EACH ROW
                        EXECUTE FUNCTION trigger_ponder_namespace_changed();
                    RAISE NOTICE 'Trigger ponder_namespace_changed created';
                END IF;
            END IF;
        END;
        $FUNC$ LANGUAGE plpgsql;
    END IF;
END;
$$;

-- Add helpful comments
COMMENT ON FUNCTION trigger_ponder_namespace_changed() IS
'Trigger function that automatically recreates ponder_events view when Ponder namespace changes.
This ensures views always point to the correct Ponder tables after a restart.';

COMMENT ON FUNCTION ensure_ponder_namespace_trigger() IS
'Helper function to create the namespace trigger if _ponder_meta table exists.
Call this after Ponder first initializes if the trigger was not created during migration.';
