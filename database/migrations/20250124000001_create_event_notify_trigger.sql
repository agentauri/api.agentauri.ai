-- ============================================================================
-- Create NOTIFY trigger for new events
-- ============================================================================
-- This trigger sends a notification on the 'new_event' channel whenever
-- a new event is inserted into the events table. This enables real-time
-- event processing without polling.
-- ============================================================================

-- Create the trigger function
CREATE OR REPLACE FUNCTION notify_new_event()
RETURNS TRIGGER AS $$
BEGIN
    -- Send notification with event ID and chain ID as payload
    -- Format: {"event_id":"<id>","chain_id":<chain_id>}
    PERFORM pg_notify(
        'new_event',
        json_build_object(
            'event_id', NEW.id,
            'chain_id', NEW.chain_id,
            'block_number', NEW.block_number,
            'event_type', NEW.event_type,
            'registry', NEW.registry
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create the trigger on events table
DROP TRIGGER IF EXISTS trigger_notify_new_event ON events;
CREATE TRIGGER trigger_notify_new_event
    AFTER INSERT ON events
    FOR EACH ROW
    EXECUTE FUNCTION notify_new_event();

-- Add index for faster event retrieval by Event Processor
CREATE INDEX IF NOT EXISTS idx_events_id_chain_id ON events(id, chain_id);

-- Comment on trigger
COMMENT ON TRIGGER trigger_notify_new_event ON events IS
'Sends PostgreSQL NOTIFY on new_event channel when events are inserted';
