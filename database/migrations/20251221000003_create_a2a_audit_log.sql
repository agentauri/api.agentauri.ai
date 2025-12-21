-- Migration: Create A2A task audit log table
-- Phase 3 Security Fix: Add audit trail for task operations
--
-- This table records all operations on A2A tasks for:
-- 1. Security auditing and compliance
-- 2. Debugging and troubleshooting
-- 3. Usage analytics and billing reconciliation

-- ============================================================================
-- Audit Log Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS a2a_task_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Task reference
    task_id UUID NOT NULL REFERENCES a2a_tasks(id) ON DELETE CASCADE,
    organization_id TEXT NOT NULL,

    -- Event details
    event_type TEXT NOT NULL,  -- 'created', 'started', 'completed', 'failed', 'cancelled', 'timeout'

    -- Actor information (who triggered the event)
    actor_type TEXT NOT NULL,  -- 'user', 'system', 'api_key'
    actor_id TEXT,             -- user_id, 'processor', or api_key prefix

    -- Event metadata
    tool TEXT,                 -- The tool being executed
    cost_micro_usdc BIGINT,    -- Cost in micro-USDC (for completed tasks)
    duration_ms BIGINT,        -- Duration in milliseconds
    error_message TEXT,        -- Error message (for failed/timeout events)

    -- Additional context
    metadata JSONB DEFAULT '{}',

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- Indexes
-- ============================================================================

-- Index for finding audit logs by task
CREATE INDEX IF NOT EXISTS idx_a2a_audit_task_id
ON a2a_task_audit_log (task_id, created_at DESC);

-- Index for finding audit logs by organization
CREATE INDEX IF NOT EXISTS idx_a2a_audit_organization
ON a2a_task_audit_log (organization_id, created_at DESC);

-- Index for finding audit logs by event type
CREATE INDEX IF NOT EXISTS idx_a2a_audit_event_type
ON a2a_task_audit_log (event_type, created_at DESC);

-- Index for analytics queries (cost aggregation)
CREATE INDEX IF NOT EXISTS idx_a2a_audit_analytics
ON a2a_task_audit_log (organization_id, event_type, created_at)
WHERE event_type = 'completed';

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE a2a_task_audit_log IS 'Audit trail for A2A task operations';
COMMENT ON COLUMN a2a_task_audit_log.event_type IS 'Type of event: created, started, completed, failed, cancelled, timeout';
COMMENT ON COLUMN a2a_task_audit_log.actor_type IS 'Type of actor: user, system, api_key';
COMMENT ON COLUMN a2a_task_audit_log.cost_micro_usdc IS 'Cost in micro-USDC (1 USDC = 1,000,000 micro-USDC)';
