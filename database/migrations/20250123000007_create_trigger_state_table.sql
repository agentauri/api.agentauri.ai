-- Migration 7: Create trigger_state Table
-- Description: Stores state for stateful triggers (EMA, counters, etc.)
-- Created: 2025-01-23

CREATE TABLE trigger_state (
    trigger_id TEXT PRIMARY KEY,
    state_data JSONB NOT NULL,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Example state_data JSONB:
-- EMA: {"ema": 72.5, "count": 15}
-- Rate counter: {"current_hour": "2025-01-23T12:00:00Z", "count": 7, "recent_timestamps": [1735689000, ...]}
