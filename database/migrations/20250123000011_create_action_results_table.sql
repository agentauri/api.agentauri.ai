-- Migration 11: Create action_results Table
-- Description: Audit trail of all action executions
-- Created: 2025-01-23

CREATE TABLE action_results (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    job_id TEXT NOT NULL, -- Redis job ID
    trigger_id TEXT,
    event_id TEXT,
    action_type TEXT NOT NULL CHECK (action_type IN ('telegram', 'rest', 'mcp')),
    status TEXT NOT NULL CHECK (status IN ('success', 'failed', 'retrying')),
    executed_at TIMESTAMPTZ DEFAULT NOW(),
    duration_ms INTEGER,
    error_message TEXT,
    response_data JSONB,
    retry_count INTEGER DEFAULT 0,
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE SET NULL
    -- Note: event_id is a reference only, no foreign key constraint due to events composite primary key
);

-- Index for finding all results for a specific trigger
CREATE INDEX idx_action_results_trigger_id ON action_results(trigger_id);

-- Index for finding all results for a specific event
CREATE INDEX idx_action_results_event_id ON action_results(event_id);

-- Index for filtering by status (e.g., finding all failed actions)
CREATE INDEX idx_action_results_status ON action_results(status);

-- Index for time-based queries and cleanup operations
CREATE INDEX idx_action_results_executed_at ON action_results(executed_at DESC);

-- Index for analyzing performance by action type
CREATE INDEX idx_action_results_action_type ON action_results(action_type);
