-- Migration 12: Create agent_mcp_tokens Table (Optional)
-- Description: Stores authentication tokens for agent MCP servers
-- Created: 2025-01-23
-- Note: Tokens should be encrypted at rest using application-level or database encryption

CREATE TABLE agent_mcp_tokens (
    agent_id BIGINT PRIMARY KEY,
    token TEXT NOT NULL, -- Should be encrypted at rest
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER update_agent_mcp_tokens_updated_at
    BEFORE UPDATE ON agent_mcp_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
