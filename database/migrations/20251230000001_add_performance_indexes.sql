-- Performance optimization indexes
-- Created: 2025-12-30
-- These indexes optimize frequently queried columns identified in performance analysis

-- 1. Organization membership lookups (used in EVERY authenticated request)
-- Query: SELECT role FROM organization_members WHERE organization_id = $1 AND user_id = $2
CREATE INDEX IF NOT EXISTS idx_org_members_org_user
ON organization_members(organization_id, user_id);

-- 2. API key lookup by prefix (critical auth path)
-- Query: SELECT * FROM api_keys WHERE prefix = $1 AND revoked_at IS NULL
CREATE INDEX IF NOT EXISTS idx_api_keys_prefix_active
ON api_keys(prefix) WHERE revoked_at IS NULL;

-- 3. Trigger conditions by trigger_id (fetched on every trigger evaluation)
-- Query: SELECT * FROM trigger_conditions WHERE trigger_id = $1
CREATE INDEX IF NOT EXISTS idx_trigger_conditions_trigger_id
ON trigger_conditions(trigger_id);

-- 4. Trigger actions by trigger_id with priority ordering
-- Query: SELECT * FROM trigger_actions WHERE trigger_id = $1 ORDER BY priority ASC
CREATE INDEX IF NOT EXISTS idx_trigger_actions_trigger_priority
ON trigger_actions(trigger_id, priority ASC, id ASC);

-- 5. Trigger state lookups for stateful triggers
-- Query: SELECT * FROM trigger_state WHERE trigger_id = $1
CREATE INDEX IF NOT EXISTS idx_trigger_state_trigger_id
ON trigger_state(trigger_id);

-- 6. Triggers by chain/registry for event matching (event processor hot path)
-- Query: SELECT * FROM triggers WHERE (chain_id = $1 OR chain_id IS NULL) AND registry = $2 AND enabled = true
CREATE INDEX IF NOT EXISTS idx_triggers_registry_enabled
ON triggers(registry, enabled) WHERE enabled = true;

-- Separate index for wildcard chain matching
CREATE INDEX IF NOT EXISTS idx_triggers_wildcard_chain
ON triggers(registry) WHERE chain_id IS NULL AND enabled = true;

-- 7. User identity lookups for social auth
CREATE INDEX IF NOT EXISTS idx_user_identities_user_provider
ON user_identities(user_id, provider);

CREATE INDEX IF NOT EXISTS idx_user_identities_provider_id
ON user_identities(provider, provider_user_id);

-- 8. Credit transactions by organization (billing queries)
CREATE INDEX IF NOT EXISTS idx_credit_transactions_org_created
ON credit_transactions(organization_id, created_at DESC);

-- 9. Used nonces lookup (auth flow)
CREATE INDEX IF NOT EXISTS idx_used_nonces_wallet_expires
ON used_nonces(wallet_address, expires_at);

-- 10. Agent follows by organization
CREATE INDEX IF NOT EXISTS idx_agent_follows_org
ON agent_follows(organization_id);

-- Analyze tables to update statistics
ANALYZE organization_members;
ANALYZE api_keys;
ANALYZE trigger_conditions;
ANALYZE trigger_actions;
ANALYZE trigger_state;
ANALYZE triggers;
ANALYZE user_identities;
ANALYZE credit_transactions;
ANALYZE used_nonces;
ANALYZE agent_follows;
