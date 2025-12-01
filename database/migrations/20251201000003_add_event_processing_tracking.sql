-- Migration: Add event processing tracking to prevent silent event loss
-- This table ensures idempotent event processing and enables polling fallback
-- for events that may have been missed during database downtime or listener disconnection.

-- Create processed_events table
-- NOTE: No foreign key constraint to events table because events has a composite primary key (id, created_at)
-- due to TimescaleDB hypertable partitioning. The relationship is enforced at application level.
CREATE TABLE processed_events (
    event_id TEXT PRIMARY KEY,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processor_instance TEXT NOT NULL,
    processing_duration_ms INTEGER,
    triggers_matched INTEGER DEFAULT 0,
    actions_enqueued INTEGER DEFAULT 0
);

-- Index for polling unprocessed events (critical for fallback performance)
CREATE INDEX idx_processed_events_processed_at ON processed_events(processed_at DESC);

-- Index for processor instance analytics
CREATE INDEX idx_processed_events_processor ON processed_events(processor_instance);

-- Helper function to check if an event has been processed
CREATE OR REPLACE FUNCTION is_event_processed(p_event_id TEXT)
RETURNS BOOLEAN AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1 FROM processed_events WHERE event_id = p_event_id
    );
END;
$$ LANGUAGE plpgsql STABLE;

-- View for monitoring unprocessed events (useful for observability)
CREATE OR REPLACE VIEW unprocessed_events AS
SELECT
    e.id,
    e.chain_id,
    e.block_number,
    e.registry,
    e.event_type,
    e.created_at,
    EXTRACT(EPOCH FROM (NOW() - e.created_at)) AS age_seconds
FROM events e
WHERE NOT EXISTS (
    SELECT 1 FROM processed_events pe WHERE pe.event_id = e.id
)
ORDER BY e.created_at ASC, e.id ASC;

-- Index on events.created_at for efficient unprocessed event queries
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at ASC);

-- Comment for documentation
COMMENT ON TABLE processed_events IS 'Tracks which events have been processed by the event-processor to enable idempotent processing and polling fallback';
COMMENT ON COLUMN processed_events.processor_instance IS 'Hostname or instance ID of the processor that handled this event (for distributed deployments)';
COMMENT ON COLUMN processed_events.processing_duration_ms IS 'Time taken to process the event in milliseconds';
COMMENT ON COLUMN processed_events.triggers_matched IS 'Number of triggers that matched this event';
COMMENT ON COLUMN processed_events.actions_enqueued IS 'Number of actions enqueued to the job queue';
