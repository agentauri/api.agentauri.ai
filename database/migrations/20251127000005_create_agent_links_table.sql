-- Migration: Create Agent Links Table
-- Description: Link on-chain agents to organizations
-- Created: 2025-11-27

-- Table: agent_links
-- Purpose: Associate ERC-8004 agents (NFTs) with organizations via wallet signature
CREATE TABLE agent_links (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    agent_id BIGINT NOT NULL,           -- ERC-8004 token ID
    chain_id INTEGER NOT NULL,          -- Blockchain chain ID
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    wallet_address TEXT NOT NULL,       -- Wallet that signed the link
    linked_by TEXT NOT NULL REFERENCES users(id),
    signature TEXT NOT NULL,            -- EIP-191 signature proving ownership
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked')),
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT agent_links_agent_chain_unique UNIQUE (agent_id, chain_id)
);

-- Indexes for common queries
CREATE INDEX idx_agent_links_organization ON agent_links(organization_id) WHERE status = 'active';
CREATE INDEX idx_agent_links_agent ON agent_links(agent_id, chain_id) WHERE status = 'active';
CREATE INDEX idx_agent_links_wallet ON agent_links(wallet_address);

-- Comment on table
COMMENT ON TABLE agent_links IS 'Links between ERC-8004 agent NFTs and organizations';
COMMENT ON COLUMN agent_links.agent_id IS 'ERC-8004 Identity Registry token ID';
COMMENT ON COLUMN agent_links.chain_id IS 'Blockchain network chain ID (e.g., 11155111 for Sepolia)';
COMMENT ON COLUMN agent_links.signature IS 'EIP-191 signature proving wallet ownership of agent';
