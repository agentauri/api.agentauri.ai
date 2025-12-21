-- =============================================================================
-- A2A Tasks Table
-- =============================================================================
-- Stores async tasks submitted via the A2A JSON-RPC protocol.
-- Part of Phase 5: Pull Layer implementation.
--
-- Reference: docs/protocols/A2A_INTEGRATION.md
-- =============================================================================

-- Create a2a_tasks table
CREATE TABLE IF NOT EXISTS a2a_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Note: organizations.id is TEXT for legacy reasons
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- Task definition
    tool TEXT NOT NULL,                    -- Query tool name (e.g., getReputationSummary)
    arguments JSONB NOT NULL DEFAULT '{}', -- Tool arguments

    -- Task lifecycle
    status TEXT NOT NULL DEFAULT 'submitted'
        CHECK (status IN ('submitted', 'working', 'completed', 'failed', 'cancelled')),
    progress DECIMAL(3, 2) DEFAULT 0       -- 0.00 to 1.00
        CHECK (progress >= 0 AND progress <= 1),

    -- Results
    result JSONB,                          -- Task result (when completed)
    error TEXT,                            -- Error message (when failed)

    -- Cost tracking
    cost DECIMAL(20, 8),                   -- Cost in USDC

    -- Timestamps
    started_at TIMESTAMPTZ,                -- When task started processing
    completed_at TIMESTAMPTZ,              -- When task finished (success or failure)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_a2a_tasks_organization_id
    ON a2a_tasks(organization_id);

CREATE INDEX IF NOT EXISTS idx_a2a_tasks_status_pending
    ON a2a_tasks(status)
    WHERE status IN ('submitted', 'working');

CREATE INDEX IF NOT EXISTS idx_a2a_tasks_created_at
    ON a2a_tasks(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_a2a_tasks_org_status
    ON a2a_tasks(organization_id, status);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_a2a_tasks_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_a2a_tasks_updated_at
    BEFORE UPDATE ON a2a_tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_a2a_tasks_updated_at();

-- Comment on table
COMMENT ON TABLE a2a_tasks IS 'A2A Protocol async tasks for JSON-RPC queries';
COMMENT ON COLUMN a2a_tasks.tool IS 'Query tool name (e.g., getReputationSummary, getMyFeedbacks)';
COMMENT ON COLUMN a2a_tasks.status IS 'Task lifecycle state: submitted -> working -> completed/failed/cancelled';
COMMENT ON COLUMN a2a_tasks.progress IS 'Task completion progress from 0.00 to 1.00';
