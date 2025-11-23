-- api.8004.dev PostgreSQL Database Schema
-- Full schema reference (concatenation of all migrations)
-- Generated: 2025-01-23
-- PostgreSQL Version: 15+
-- Required Extensions: timescaledb, pgcrypto

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- HELPER FUNCTIONS
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- CORE TABLES
-- ============================================================================

-- Users Table
-- Stores user accounts for API authentication
CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Triggers Table
-- Stores user-defined trigger configurations
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

CREATE INDEX idx_triggers_user_id ON triggers(user_id);
CREATE INDEX idx_triggers_chain_registry_enabled
    ON triggers(chain_id, registry, enabled)
    WHERE enabled = true;

CREATE TRIGGER update_triggers_updated_at
    BEFORE UPDATE ON triggers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger Conditions Table
-- Defines matching conditions for triggers
CREATE TABLE trigger_conditions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    condition_type TEXT NOT NULL,
    field TEXT NOT NULL,
    operator TEXT NOT NULL,
    value TEXT NOT NULL,
    config JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

CREATE INDEX idx_trigger_conditions_trigger_id ON trigger_conditions(trigger_id);

-- Trigger Actions Table
-- Defines actions to execute when triggers match
CREATE TABLE trigger_actions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    action_type TEXT NOT NULL CHECK (action_type IN ('telegram', 'rest', 'mcp')),
    priority INTEGER DEFAULT 1,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

CREATE INDEX idx_trigger_actions_trigger_id ON trigger_actions(trigger_id);

-- Trigger State Table
-- Stores state for stateful triggers
CREATE TABLE trigger_state (
    trigger_id TEXT PRIMARY KEY,
    state_data JSONB NOT NULL,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Events Table
-- Immutable log of all blockchain events
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    event_type TEXT NOT NULL,
    agent_id BIGINT,
    timestamp BIGINT NOT NULL,
    owner TEXT,
    token_uri TEXT,
    metadata_key TEXT,
    metadata_value TEXT,
    client_address TEXT,
    feedback_index BIGINT,
    score INTEGER,
    tag1 TEXT,
    tag2 TEXT,
    file_uri TEXT,
    file_hash TEXT,
    validator_address TEXT,
    request_hash TEXT,
    response INTEGER,
    response_uri TEXT,
    response_hash TEXT,
    tag TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION notify_new_event()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('new_event', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER events_notify_trigger
    AFTER INSERT ON events
    FOR EACH ROW
    EXECUTE FUNCTION notify_new_event();

-- Convert to TimescaleDB hypertable
SELECT create_hypertable('events', 'created_at', chunk_time_interval => INTERVAL '7 days');

-- Events indexes
CREATE INDEX idx_events_chain_id_created_at ON events(chain_id, created_at DESC);
CREATE INDEX idx_events_agent_id ON events(agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_events_registry_type ON events(registry, event_type);
CREATE INDEX idx_events_client_address ON events(client_address) WHERE client_address IS NOT NULL;
CREATE INDEX idx_events_validator_address ON events(validator_address) WHERE validator_address IS NOT NULL;
CREATE INDEX idx_events_block_number ON events(chain_id, block_number);

-- Checkpoints Table
-- Tracks last processed block per chain
CREATE TABLE checkpoints (
    chain_id INTEGER PRIMARY KEY,
    last_block_number BIGINT NOT NULL,
    last_block_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Action Results Table
-- Audit trail of all action executions
CREATE TABLE action_results (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    job_id TEXT NOT NULL,
    trigger_id TEXT,
    event_id TEXT,
    action_type TEXT NOT NULL CHECK (action_type IN ('telegram', 'rest', 'mcp')),
    status TEXT NOT NULL CHECK (status IN ('success', 'failed', 'retrying')),
    executed_at TIMESTAMPTZ DEFAULT NOW(),
    duration_ms INTEGER,
    error_message TEXT,
    response_data JSONB,
    retry_count INTEGER DEFAULT 0,
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE SET NULL,
    CONSTRAINT fk_event FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE SET NULL
);

CREATE INDEX idx_action_results_trigger_id ON action_results(trigger_id);
CREATE INDEX idx_action_results_event_id ON action_results(event_id);
CREATE INDEX idx_action_results_status ON action_results(status);
CREATE INDEX idx_action_results_executed_at ON action_results(executed_at DESC);
CREATE INDEX idx_action_results_action_type ON action_results(action_type);

-- Agent MCP Tokens Table (Optional)
-- Stores authentication tokens for agent MCP servers
CREATE TABLE agent_mcp_tokens (
    agent_id BIGINT PRIMARY KEY,
    token TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TRIGGER update_agent_mcp_tokens_updated_at
    BEFORE UPDATE ON agent_mcp_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- MATERIALIZED VIEWS (Optional - for analytics)
-- ============================================================================

-- Hourly event statistics
CREATE MATERIALIZED VIEW events_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', created_at) AS hour,
    chain_id,
    registry,
    event_type,
    COUNT(*) as event_count
FROM events
GROUP BY hour, chain_id, registry, event_type
WITH NO DATA;

-- Refresh policy (auto-refresh every hour)
SELECT add_continuous_aggregate_policy('events_hourly',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- Hourly action execution metrics
CREATE MATERIALIZED VIEW action_metrics_hourly AS
SELECT
    DATE_TRUNC('hour', executed_at) as hour,
    action_type,
    COUNT(*) as total_executions,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as success_count,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failure_count,
    AVG(duration_ms) as avg_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_duration_ms
FROM action_results
GROUP BY hour, action_type;
