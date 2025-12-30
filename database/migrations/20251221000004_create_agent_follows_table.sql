-- Migration: Create Agent Follows Table
-- Description: Simplified interface for following all activities of an ERC-8004 agent
-- across identity, reputation, and validation registries with a single API call.

CREATE TABLE IF NOT EXISTS agent_follows (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,

    -- Target agent identification
    agent_id BIGINT NOT NULL,           -- ERC-8004 token ID being followed
    chain_id INTEGER NOT NULL,          -- Blockchain chain ID

    -- Organization scope
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Underlying trigger IDs (created and managed by the system)
    trigger_identity_id TEXT NOT NULL REFERENCES triggers(id) ON DELETE CASCADE,
    trigger_reputation_id TEXT NOT NULL REFERENCES triggers(id) ON DELETE CASCADE,
    trigger_validation_id TEXT NOT NULL REFERENCES triggers(id) ON DELETE CASCADE,

    -- Follow configuration
    enabled BOOLEAN NOT NULL DEFAULT true,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints: one follow per agent per chain per organization
    CONSTRAINT agent_follows_unique UNIQUE (agent_id, chain_id, organization_id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_agent_follows_org_enabled
    ON agent_follows(organization_id) WHERE enabled = true;

CREATE INDEX IF NOT EXISTS idx_agent_follows_agent_chain
    ON agent_follows(agent_id, chain_id);

CREATE INDEX IF NOT EXISTS idx_agent_follows_user
    ON agent_follows(user_id);

-- Auto-update updated_at timestamp
DROP TRIGGER IF EXISTS update_agent_follows_updated_at ON agent_follows;
CREATE TRIGGER update_agent_follows_updated_at
    BEFORE UPDATE ON agent_follows
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE agent_follows IS 'Simplified agent following - creates 3 triggers under the hood for identity, reputation, and validation registries';
COMMENT ON COLUMN agent_follows.agent_id IS 'ERC-8004 Identity Registry token ID being followed';
COMMENT ON COLUMN agent_follows.trigger_identity_id IS 'Auto-managed trigger for identity registry events';
COMMENT ON COLUMN agent_follows.trigger_reputation_id IS 'Auto-managed trigger for reputation registry events';
COMMENT ON COLUMN agent_follows.trigger_validation_id IS 'Auto-managed trigger for validation registry events';
