-- Migration 4: Create triggers Table
-- Description: Stores user-defined trigger configurations for event monitoring
-- Created: 2025-01-23

CREATE TABLE triggers (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    chain_id INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    enabled BOOLEAN DEFAULT true,
    is_stateful BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for finding all triggers belonging to a user
CREATE INDEX idx_triggers_user_id ON triggers(user_id);

-- Composite partial index for common query pattern: finding enabled triggers by chain and registry
-- Partial index (WHERE enabled = true) reduces index size and improves performance
CREATE INDEX idx_triggers_chain_registry_enabled
    ON triggers(chain_id, registry, enabled)
    WHERE enabled = true;

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER update_triggers_updated_at
    BEFORE UPDATE ON triggers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
