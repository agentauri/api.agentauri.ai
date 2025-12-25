-- ============================================================================
-- Migration: Fix Ponder Views to Use Dynamic Namespace
-- ============================================================================
-- PROBLEM: The ponder_events view was hardcoded to use 'ponder."Event"' schema
-- but Ponder actually writes to 'public."<namespace>__Event"' where namespace
-- is a 4-character hash (e.g., 'c9ba').
--
-- SOLUTION: Create a function to dynamically find the active Ponder namespace
-- and update the view creation to use the correct table.
--
-- NOTE: The view must cast numeric(78,0) columns to integer/bigint to match
-- the Rust Event model types (i32 for chain_id, i64 for block_number, etc.)
--
-- Created: 2025-12-25
-- ============================================================================

-- Function to get the active Ponder namespace
CREATE OR REPLACE FUNCTION get_ponder_namespace()
RETURNS TEXT AS $$
DECLARE
    namespace TEXT;
    table_name TEXT;
BEGIN
    -- Try to find namespace from _ponder_meta table
    SELECT value INTO namespace
    FROM public._ponder_meta
    WHERE key = 'app'
    LIMIT 1;

    IF namespace IS NOT NULL THEN
        RETURN namespace;
    END IF;

    -- Fallback: find namespace from table names
    SELECT tablename INTO table_name
    FROM pg_tables
    WHERE schemaname = 'public'
      AND tablename LIKE '%__Event'
      AND tablename NOT LIKE '%_reorg__%'
    ORDER BY tablename DESC
    LIMIT 1;

    IF table_name IS NOT NULL THEN
        -- Extract namespace from "xxxx__Event"
        RETURN split_part(table_name, '__', 1);
    END IF;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to recreate ponder_events view with current namespace
-- This must be called after Ponder initializes (creates its tables)
CREATE OR REPLACE FUNCTION recreate_ponder_events_view()
RETURNS void AS $$
DECLARE
    namespace TEXT;
    view_sql TEXT;
BEGIN
    namespace := get_ponder_namespace();

    IF namespace IS NULL THEN
        RAISE NOTICE 'No Ponder namespace found, skipping view recreation';
        RETURN;
    END IF;

    RAISE NOTICE 'Recreating ponder_events view for namespace: %', namespace;

    -- Drop existing views
    DROP VIEW IF EXISTS unprocessed_events CASCADE;
    DROP VIEW IF EXISTS ponder_events CASCADE;

    -- Create ponder_events view with dynamic namespace
    -- IMPORTANT: Cast numeric(78,0) columns to match Rust model types:
    --   chain_id -> integer (i32)
    --   block_number, agent_id, timestamp, feedback_index -> bigint (i64)
    view_sql := format($VIEW$
        CREATE OR REPLACE VIEW ponder_events AS
        SELECT
            id,
            chain_id::integer AS chain_id,
            block_number::bigint AS block_number,
            block_hash,
            transaction_hash,
            log_index,
            registry,
            event_type,
            agent_id::bigint AS agent_id,
            timestamp::bigint AS timestamp,
            owner,
            token_uri,
            metadata_key,
            metadata_value,
            client_address,
            feedback_index::bigint AS feedback_index,
            score,
            tag1,
            tag2,
            file_uri,
            file_hash,
            validator_address,
            request_hash,
            response,
            response_uri,
            response_hash,
            tag,
            to_timestamp(timestamp::bigint) AS created_at
        FROM public."%s__Event"
    $VIEW$, namespace);

    EXECUTE view_sql;

    -- Recreate unprocessed_events view
    CREATE OR REPLACE VIEW unprocessed_events AS
    SELECT
        e.id,
        e.chain_id,
        e.block_number,
        e.registry,
        e.event_type,
        e.created_at,
        EXTRACT(EPOCH FROM (NOW() - e.created_at)) AS age_seconds
    FROM ponder_events e
    WHERE NOT EXISTS (
        SELECT 1 FROM processed_events pe WHERE pe.event_id = e.id
    )
    ORDER BY e.created_at ASC, e.id ASC;

    RAISE NOTICE 'Successfully recreated ponder_events view for namespace: %', namespace;
END;
$$ LANGUAGE plpgsql;

-- Execute the view recreation immediately
SELECT recreate_ponder_events_view();

-- Add comment for documentation
COMMENT ON FUNCTION get_ponder_namespace() IS
'Returns the active Ponder namespace (4-char hash) from _ponder_meta or table inspection';

COMMENT ON FUNCTION recreate_ponder_events_view() IS
'Recreates ponder_events and unprocessed_events views with the current Ponder namespace.
Call this function after Ponder restarts and creates new tables with a different namespace.
IMPORTANT: This function must be called whenever Ponder is redeployed, as it may create
a new namespace. A cron job or startup hook should trigger this.';
