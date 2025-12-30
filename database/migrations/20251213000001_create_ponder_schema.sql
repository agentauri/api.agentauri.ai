-- ============================================================================
-- Migration: Create Ponder Schema for Shared Event Indexing
-- ============================================================================
-- This migration creates a dedicated schema for Ponder blockchain indexer.
-- Ponder indexes blockchain events which are public and immutable, so we can
-- share a single Ponder instance across staging and production environments.
--
-- Architecture:
--   - Ponder writes to 'ponder' schema (Event, Checkpoint tables)
--   - Backend services read from 'ponder' schema (read-only)
--   - NOTIFY trigger sends events to event-processor
--
-- Created: 2025-12-13
-- ============================================================================

-- Create the ponder schema
CREATE SCHEMA IF NOT EXISTS ponder;

-- Grant usage on schema to the application user
-- (The same user used by Ponder and backend services)
GRANT USAGE ON SCHEMA ponder TO CURRENT_USER;

-- Grant all privileges for Ponder to create and manage tables
GRANT ALL PRIVILEGES ON SCHEMA ponder TO CURRENT_USER;

-- Ensure future tables in ponder schema are accessible
ALTER DEFAULT PRIVILEGES IN SCHEMA ponder
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO CURRENT_USER;

-- Create the notify function that will be used by the trigger
-- This function sends a JSON payload with event details for real-time processing
CREATE OR REPLACE FUNCTION ponder.notify_ponder_event()
RETURNS TRIGGER AS $$
BEGIN
    -- Send notification with event details matching the existing format
    -- Note: Ponder uses camelCase column names, we map them to the expected format
    PERFORM pg_notify(
        'new_event',
        json_build_object(
            'event_id', NEW.id,
            'chain_id', NEW."chainId",
            'block_number', NEW."blockNumber",
            'event_type', NEW."eventType",
            'registry', NEW.registry
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add comment for documentation
COMMENT ON SCHEMA ponder IS 'Dedicated schema for Ponder blockchain indexer - shared across environments';
COMMENT ON FUNCTION ponder.notify_ponder_event() IS 'Sends PostgreSQL NOTIFY on new_event channel when Ponder inserts events';

-- Note: The trigger on ponder."Event" table will be created by a separate
-- migration (20251213000002_create_ponder_notify_trigger.sql) that runs
-- AFTER Ponder has created its tables.
