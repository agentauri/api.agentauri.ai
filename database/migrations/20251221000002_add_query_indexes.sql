-- Migration: Add missing indexes for A2A queries and task processing
-- Phase 2 Security Fix: Optimize query performance
--
-- NOTE: a2a_tasks already has comprehensive indexes from its creation migration.
-- This migration adds complementary indexes for specific query patterns.
--
-- Existing a2a_tasks indexes (from 20251221000001):
-- - idx_a2a_tasks_created_at (created_at DESC)
-- - idx_a2a_tasks_org_status (organization_id, status)
-- - idx_a2a_tasks_organization_id (organization_id)
-- - idx_a2a_tasks_status_pending (status) WHERE status IN ('submitted', 'working')

-- ============================================================================
-- A2A Tasks - Optimized claim index
-- ============================================================================

-- Index specifically optimized for the task claiming query pattern:
-- SELECT id FROM a2a_tasks WHERE status = 'submitted' ORDER BY created_at ASC LIMIT N
-- This partial index is smaller and faster than a full index
CREATE INDEX IF NOT EXISTS idx_a2a_tasks_claim_optimized
ON a2a_tasks (created_at ASC)
WHERE status = 'submitted';

-- ============================================================================
-- Credits Indexes (for credit validation)
-- ============================================================================

-- Index for credit balance lookup by organization (if not already exists)
CREATE INDEX IF NOT EXISTS idx_credits_organization
ON credits (organization_id);

-- ============================================================================
-- Ponder Events Indexes
-- These will be created when ponder_events table exists (created by Ponder indexer)
-- ============================================================================

-- NOTE: The following indexes will be created by a separate migration or
-- manually when the Ponder indexer has created the ponder_events table.
-- They are documented here for reference:
--
-- CREATE INDEX IF NOT EXISTS idx_ponder_events_feedback
-- ON ponder_events (agent_id, timestamp DESC)
-- WHERE registry = 'reputation' AND event_type = 'NewFeedback';
--
-- CREATE INDEX IF NOT EXISTS idx_ponder_events_agent_created
-- ON ponder_events (agent_id, timestamp DESC)
-- WHERE registry = 'identity' AND event_type = 'AgentCreated';
--
-- CREATE INDEX IF NOT EXISTS idx_ponder_events_validation
-- ON ponder_events (agent_id, timestamp DESC)
-- WHERE registry = 'validation';

-- ============================================================================
-- Comment on migration
-- ============================================================================

COMMENT ON INDEX idx_a2a_tasks_claim_optimized IS 'Optimizes FOR UPDATE SKIP LOCKED task claiming with created_at ordering';
COMMENT ON INDEX idx_credits_organization IS 'Optimizes credit balance lookup by organization';
