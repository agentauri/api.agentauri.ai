-- Test Data Seed File
-- Purpose: Populate database with sample data for development and testing
-- Usage: psql -d agentauri_backend -f database/seeds/test_data.sql
-- Note: This file is safe to run multiple times (uses ON CONFLICT DO NOTHING)

-- ============================================================================
-- TEST USERS
-- ============================================================================

-- Insert test users
-- Password for all test users: "password123" (bcrypt hash)
INSERT INTO users (id, username, email, password_hash, is_active, created_at)
VALUES
    ('test-user-1', 'alice', 'alice@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqgFV0jz1q', true, NOW() - INTERVAL '30 days'),
    ('test-user-2', 'bob', 'bob@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqgFV0jz1q', true, NOW() - INTERVAL '15 days'),
    ('test-user-3', 'charlie', 'charlie@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqgFV0jz1q', false, NOW() - INTERVAL '7 days')
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- TEST TRIGGERS
-- ============================================================================

-- Test trigger 1: Monitor high reputation scores on Base Sepolia
INSERT INTO triggers (id, user_id, name, description, chain_id, registry, enabled, is_stateful, created_at)
VALUES
    ('trigger-1', 'test-user-1', 'High Reputation Alert', 'Alert when agent receives reputation score > 80', 84532, 'reputation', true, false, NOW() - INTERVAL '20 days')
ON CONFLICT (id) DO NOTHING;

-- Conditions for trigger 1
INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value, created_at)
VALUES
    ((SELECT id FROM triggers WHERE id = 'trigger-1'), 'score_threshold', 'score', '>', '80', NOW() - INTERVAL '20 days')
ON CONFLICT DO NOTHING;

-- Actions for trigger 1
INSERT INTO trigger_actions (trigger_id, action_type, priority, config, created_at)
VALUES
    ((SELECT id FROM triggers WHERE id = 'trigger-1'), 'telegram', 1,
     '{"chat_id": "123456789", "message_template": "High reputation score detected!\nAgent: {{agent_id}}\nScore: {{score}}\nClient: {{client_address}}", "parse_mode": "Markdown"}'::jsonb,
     NOW() - INTERVAL '20 days')
ON CONFLICT DO NOTHING;

-- Test trigger 2: EMA-based stateful trigger
INSERT INTO triggers (id, user_id, name, description, chain_id, registry, enabled, is_stateful, created_at)
VALUES
    ('trigger-2', 'test-user-1', 'EMA Score Monitor', 'Track exponential moving average of reputation scores', 84532, 'reputation', true, true, NOW() - INTERVAL '10 days')
ON CONFLICT (id) DO NOTHING;

-- Conditions for trigger 2
INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value, config, created_at)
VALUES
    ((SELECT id FROM triggers WHERE id = 'trigger-2'), 'ema_threshold', 'score', '>', '75',
     '{"window_size": 10, "alpha": 0.2}'::jsonb,
     NOW() - INTERVAL '10 days')
ON CONFLICT DO NOTHING;

-- Actions for trigger 2
INSERT INTO trigger_actions (trigger_id, action_type, priority, config, created_at)
VALUES
    ((SELECT id FROM triggers WHERE id = 'trigger-2'), 'rest', 1,
     '{"method": "POST", "url": "https://webhook.example.com/ema-alert", "headers": {"Content-Type": "application/json"}, "body_template": {"agent_id": "{{agent_id}}", "ema": "{{ema}}", "score": "{{score}}"}, "timeout_ms": 5000}'::jsonb,
     NOW() - INTERVAL '10 days')
ON CONFLICT DO NOTHING;

-- State for trigger 2
INSERT INTO trigger_state (trigger_id, state_data, last_updated)
VALUES
    ('trigger-2', '{"ema": 72.5, "count": 15}'::jsonb, NOW() - INTERVAL '1 hour')
ON CONFLICT (trigger_id) DO UPDATE SET state_data = EXCLUDED.state_data;

-- Test trigger 3: Identity token creation monitor (disabled)
INSERT INTO triggers (id, user_id, name, description, chain_id, registry, enabled, is_stateful, created_at)
VALUES
    ('trigger-3', 'test-user-2', 'Token Creation Monitor', 'Monitor new agent token creations', 84532, 'identity', false, false, NOW() - INTERVAL '5 days')
ON CONFLICT (id) DO NOTHING;

-- Conditions for trigger 3
INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value, created_at)
VALUES
    ((SELECT id FROM triggers WHERE id = 'trigger-3'), 'event_type_equals', 'event_type', '=', 'TokenCreated', NOW() - INTERVAL '5 days')
ON CONFLICT DO NOTHING;

-- Actions for trigger 3
INSERT INTO trigger_actions (trigger_id, action_type, priority, config, created_at)
VALUES
    ((SELECT id FROM triggers WHERE id = 'trigger-3'), 'mcp', 1,
     '{"resolve_endpoint": true, "tool_name": "agent.notifyTokenCreation", "verify_file_hash": false}'::jsonb,
     NOW() - INTERVAL '5 days')
ON CONFLICT DO NOTHING;

-- ============================================================================
-- TEST EVENTS
-- ============================================================================

-- Sample reputation events
INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index, registry, event_type, agent_id, timestamp, client_address, feedback_index, score, tag1, tag2, file_uri, file_hash, created_at)
VALUES
    ('84532-1000-0', 84532, 1000, '0xabc123', '0xdef456', 0, 'reputation', 'FeedbackProvided', 42, 1737600000, '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1', 1, 85, 'helpful', 'fast', 'ipfs://Qm123', '0xhash1', NOW() - INTERVAL '5 days'),
    ('84532-1001-0', 84532, 1001, '0xabc124', '0xdef457', 0, 'reputation', 'FeedbackProvided', 42, 1737600060, '0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199', 2, 90, 'excellent', 'reliable', 'ipfs://Qm124', '0xhash2', NOW() - INTERVAL '4 days'),
    ('84532-1002-0', 84532, 1002, '0xabc125', '0xdef458', 0, 'reputation', 'FeedbackProvided', 99, 1737600120, '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1', 3, 65, 'slow', 'accurate', 'ipfs://Qm125', '0xhash3', NOW() - INTERVAL '3 days'),
    ('84532-1003-0', 84532, 1003, '0xabc126', '0xdef459', 0, 'reputation', 'FeedbackProvided', 42, 1737600180, '0x5c6B0f7Bf3E7ce046039Bd8FABdfD3f9F5021678', 4, 88, 'professional', 'detailed', 'ipfs://Qm126', '0xhash4', NOW() - INTERVAL '2 days')
ON CONFLICT (id) DO NOTHING;

-- Sample identity events
INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index, registry, event_type, agent_id, timestamp, owner, token_uri, created_at)
VALUES
    ('84532-2000-0', 84532, 2000, '0xabc200', '0xdef500', 0, 'identity', 'TokenCreated', 42, 1737500000, '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1', 'ipfs://QmAgent42', NOW() - INTERVAL '10 days'),
    ('84532-2001-0', 84532, 2001, '0xabc201', '0xdef501', 0, 'identity', 'TokenCreated', 99, 1737500060, '0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199', 'ipfs://QmAgent99', NOW() - INTERVAL '8 days')
ON CONFLICT (id) DO NOTHING;

-- Sample validation events
INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index, registry, event_type, agent_id, timestamp, validator_address, request_hash, response, response_uri, response_hash, tag, created_at)
VALUES
    ('84532-3000-0', 84532, 3000, '0xabc300', '0xdef600', 0, 'validation', 'ResponseProvided', 42, 1737550000, '0x5c6B0f7Bf3E7ce046039Bd8FABdfD3f9F5021678', '0xreq1', 1, 'ipfs://QmResp1', '0xresphash1', 'verified', NOW() - INTERVAL '6 days'),
    ('84532-3001-0', 84532, 3001, '0xabc301', '0xdef601', 0, 'validation', 'ResponseProvided', 42, 1737550060, '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1', '0xreq2', 1, 'ipfs://QmResp2', '0xresphash2', 'approved', NOW() - INTERVAL '5 days')
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- TEST CHECKPOINTS
-- ============================================================================

-- Sample checkpoints for Base Sepolia
INSERT INTO checkpoints (chain_id, last_block_number, last_block_hash, updated_at)
VALUES
    (84532, 3001, '0xabc301', NOW())
ON CONFLICT (chain_id) DO UPDATE SET
    last_block_number = EXCLUDED.last_block_number,
    last_block_hash = EXCLUDED.last_block_hash,
    updated_at = EXCLUDED.updated_at;

-- ============================================================================
-- TEST ACTION RESULTS
-- ============================================================================

-- Sample action results
INSERT INTO action_results (id, job_id, trigger_id, event_id, action_type, status, executed_at, duration_ms, response_data, retry_count)
VALUES
    ('result-1', 'job-123', 'trigger-1', '84532-1000-0', 'telegram', 'success', NOW() - INTERVAL '5 days', 250, '{"message_id": 12345}'::jsonb, 0),
    ('result-2', 'job-124', 'trigger-1', '84532-1001-0', 'telegram', 'success', NOW() - INTERVAL '4 days', 180, '{"message_id": 12346}'::jsonb, 0),
    ('result-3', 'job-125', 'trigger-2', '84532-1002-0', 'rest', 'failed', NOW() - INTERVAL '3 days', 5020, '{"error": "timeout"}'::jsonb, 3),
    ('result-4', 'job-126', 'trigger-1', '84532-1003-0', 'telegram', 'success', NOW() - INTERVAL '2 days', 195, '{"message_id": 12347}'::jsonb, 0)
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- TEST MCP TOKENS
-- ============================================================================

-- Sample MCP tokens (should be encrypted in production)
INSERT INTO agent_mcp_tokens (agent_id, token, created_at)
VALUES
    (42, 'test-token-agent-42-abc123def456', NOW() - INTERVAL '10 days'),
    (99, 'test-token-agent-99-xyz789uvw012', NOW() - INTERVAL '8 days')
ON CONFLICT (agent_id) DO UPDATE SET token = EXCLUDED.token;

-- ============================================================================
-- VERIFICATION QUERIES
-- ============================================================================

-- Uncomment to verify data was inserted correctly:
-- SELECT 'Users:', COUNT(*) FROM users;
-- SELECT 'Triggers:', COUNT(*) FROM triggers;
-- SELECT 'Trigger Conditions:', COUNT(*) FROM trigger_conditions;
-- SELECT 'Trigger Actions:', COUNT(*) FROM trigger_actions;
-- SELECT 'Events:', COUNT(*) FROM events;
-- SELECT 'Action Results:', COUNT(*) FROM action_results;
-- SELECT 'Checkpoints:', COUNT(*) FROM checkpoints;
-- SELECT 'MCP Tokens:', COUNT(*) FROM agent_mcp_tokens;
