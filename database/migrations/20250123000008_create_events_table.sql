-- Migration 8: Create events Table
-- Description: Immutable log of all blockchain events from Ponder indexers
-- Created: 2025-01-23
-- Note: This creates the table only. TimescaleDB hypertable conversion happens in next migration.

CREATE TABLE events (
    id TEXT NOT NULL, -- Format: {chain_id}-{block_number}-{log_index}
    chain_id INTEGER NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    event_type TEXT NOT NULL,

    -- Common fields
    agent_id BIGINT,
    timestamp BIGINT NOT NULL, -- Block timestamp (Unix seconds)

    -- Identity Registry fields
    owner TEXT,
    token_uri TEXT,
    metadata_key TEXT,
    metadata_value TEXT,

    -- Reputation Registry fields
    client_address TEXT,
    feedback_index BIGINT,
    score INTEGER,
    tag1 TEXT,
    tag2 TEXT,
    file_uri TEXT,
    file_hash TEXT,

    -- Validation Registry fields
    validator_address TEXT,
    request_hash TEXT,
    response INTEGER,
    response_uri TEXT,
    response_hash TEXT,
    tag TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Composite primary key including partitioning column for TimescaleDB compatibility
    PRIMARY KEY (id, created_at)
);

-- Function to notify about new events via PostgreSQL NOTIFY
-- This allows real-time event processing without polling
CREATE OR REPLACE FUNCTION notify_new_event()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('new_event', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to send notification for each new event
CREATE TRIGGER events_notify_trigger
    AFTER INSERT ON events
    FOR EACH ROW
    EXECUTE FUNCTION notify_new_event();
