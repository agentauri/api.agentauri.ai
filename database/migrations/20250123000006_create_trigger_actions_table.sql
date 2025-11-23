-- Migration 6: Create trigger_actions Table
-- Description: Defines actions to execute when triggers match events
-- Created: 2025-01-23

CREATE TABLE trigger_actions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    action_type TEXT NOT NULL CHECK (action_type IN ('telegram', 'rest', 'mcp')),
    priority INTEGER DEFAULT 1, -- Lower number = higher priority
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Index for efficiently fetching all actions for a trigger
CREATE INDEX idx_trigger_actions_trigger_id ON trigger_actions(trigger_id);

-- Example config JSONB:
-- Telegram: {"chat_id": "123456789", "message_template": "...", "parse_mode": "Markdown"}
-- REST: {"method": "POST", "url": "...", "headers": {...}, "body_template": {...}, "timeout_ms": 30000}
-- MCP: {"resolve_endpoint": true, "tool_name": "agent.receiveFeedback", "verify_file_hash": true, ...}
