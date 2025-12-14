-- Mark all existing Ponder events as processed to prevent re-sending historical notifications
-- This is a one-time fix for the migration to the new Ponder schema

INSERT INTO processed_events (event_id, processed_at, processor_instance, triggers_matched, actions_enqueued)
SELECT
    id,
    NOW(),
    'migration-20251214-historical-fix',
    0,
    0
FROM ponder."Event"
WHERE NOT EXISTS (
    SELECT 1 FROM processed_events WHERE event_id = ponder."Event".id
)
ON CONFLICT (event_id) DO NOTHING;
