-- Migration 5: Create trigger_conditions Table
-- Description: Defines matching conditions for triggers
-- Created: 2025-01-23

CREATE TABLE trigger_conditions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    condition_type TEXT NOT NULL,
    field TEXT NOT NULL,
    operator TEXT NOT NULL,
    value TEXT NOT NULL,
    config JSONB, -- Extra configuration (window_size, alpha, etc.)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Index for efficiently fetching all conditions for a trigger
CREATE INDEX idx_trigger_conditions_trigger_id ON trigger_conditions(trigger_id);

-- Example condition_type values:
-- 'agent_id_equals', 'score_threshold', 'tag_equals',
-- 'validator_whitelist', 'event_type_equals',
-- 'ema_threshold', 'rate_limit', 'file_uri_exists'

-- Example config JSONB:
-- For EMA: {"window_size": 10, "alpha": 0.2}
-- For rate limit: {"time_window": "1h", "reset_on_trigger": true}
