-- ============================================================================
-- Migration: Fix get_ponder_namespace() to Use Live Key
-- ============================================================================
-- PROBLEM: The function was looking for key='app' but Ponder stores the active
-- namespace in key='live' as {"instance_id":"xxxx"}
--
-- The _ponder_meta table has:
--   key='live', value='{"instance_id":"ac57"}'  <- This is the ACTIVE namespace
--   key='app_c9ba', value='{...}'               <- Old namespace metadata
--   key='app_ac57', value='{...}'               <- Current namespace metadata
--
-- Created: 2026-01-02
-- ============================================================================

-- Fix the function to use the correct key
CREATE OR REPLACE FUNCTION get_ponder_namespace()
RETURNS TEXT AS $$
DECLARE
    namespace TEXT;
    live_value TEXT;
    table_name TEXT;
BEGIN
    -- First priority: Get namespace from 'live' key which contains {"instance_id":"xxxx"}
    SELECT value INTO live_value
    FROM public._ponder_meta
    WHERE key = 'live'
    LIMIT 1;

    IF live_value IS NOT NULL THEN
        -- Extract instance_id from JSON: {"instance_id":"ac57"} -> "ac57"
        namespace := live_value::json->>'instance_id';
        IF namespace IS NOT NULL THEN
            RETURN namespace;
        END IF;
    END IF;

    -- Fallback: find namespace from table names (most recent by count)
    SELECT split_part(tablename, '__', 1) INTO namespace
    FROM pg_tables
    WHERE schemaname = 'public'
      AND tablename LIKE '%__Event'
      AND tablename NOT LIKE '%_reorg__%'
    ORDER BY (
        SELECT reltuples::bigint
        FROM pg_class
        WHERE relname = tablename
    ) DESC
    LIMIT 1;

    RETURN namespace;
END;
$$ LANGUAGE plpgsql STABLE;

-- Recreate the views with the correct namespace
SELECT recreate_ponder_events_view();

-- Add updated comment
COMMENT ON FUNCTION get_ponder_namespace() IS
'Returns the active Ponder namespace from _ponder_meta.live.instance_id.
Fixed on 2026-01-02 to use the correct key (live instead of app).';
