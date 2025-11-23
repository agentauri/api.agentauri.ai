# Database Schema

## Overview

The api.8004.dev backend uses PostgreSQL 15+ with TimescaleDB extension for efficient time-series data management. The schema is designed for high-performance event processing, flexible trigger definitions, and comprehensive audit trails.

## Database Configuration

```sql
-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS pgcrypto; -- For UUID generation

-- Create database (if not exists)
-- createdb erc8004_backend
```

## Schema Design Principles

1. **Immutability**: Events table is append-only (no updates/deletes)
2. **Audit Trail**: All tables include created_at, updated_at timestamps
3. **Referential Integrity**: Foreign keys with appropriate CASCADE rules
4. **Performance**: Indexes on common query patterns
5. **Flexibility**: JSONB columns for extensible configurations
6. **Time-Series Optimization**: TimescaleDB hypertables for events

## Core Tables

### users

Stores user accounts for API authentication.

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL, -- bcrypt hash
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

### triggers

Stores user-defined trigger configurations.

```sql
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

-- Indexes for common queries
CREATE INDEX idx_triggers_user_id ON triggers(user_id);
CREATE INDEX idx_triggers_chain_registry_enabled
    ON triggers(chain_id, registry, enabled)
    WHERE enabled = true;

CREATE TRIGGER update_triggers_updated_at
    BEFORE UPDATE ON triggers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

### trigger_conditions

Defines matching conditions for triggers.

```sql
CREATE TABLE trigger_conditions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    condition_type TEXT NOT NULL,
    field TEXT NOT NULL,
    operator TEXT NOT NULL,
    value TEXT NOT NULL,
    config JSONB, -- Extra configuration (window_size, alpha, etc.)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

CREATE INDEX idx_trigger_conditions_trigger_id ON trigger_conditions(trigger_id);

-- Example condition_type values:
-- 'agent_id_equals', 'score_threshold', 'tag_equals',
-- 'validator_whitelist', 'event_type_equals',
-- 'ema_threshold', 'rate_limit', 'file_uri_exists'

-- Example config JSONB:
-- For EMA: {"window_size": 10, "alpha": 0.2}
-- For rate limit: {"time_window": "1h", "reset_on_trigger": true}
```

### trigger_actions

Defines actions to execute when triggers match.

```sql
CREATE TABLE trigger_actions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    action_type TEXT NOT NULL CHECK (action_type IN ('telegram', 'rest', 'mcp')),
    priority INTEGER DEFAULT 1, -- Lower number = higher priority
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

CREATE INDEX idx_trigger_actions_trigger_id ON trigger_actions(trigger_id);

-- Example config JSONB:
-- Telegram: {"chat_id": "123456789", "message_template": "...", "parse_mode": "Markdown"}
-- REST: {"method": "POST", "url": "...", "headers": {...}, "body_template": {...}, "timeout_ms": 30000}
-- MCP: {"resolve_endpoint": true, "tool_name": "agent.receiveFeedback", "verify_file_hash": true, ...}
```

### trigger_state

Stores state for stateful triggers (EMA, counters, etc.).

```sql
CREATE TABLE trigger_state (
    trigger_id TEXT PRIMARY KEY,
    state_data JSONB NOT NULL,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Example state_data JSONB:
-- EMA: {"ema": 72.5, "count": 15}
-- Rate counter: {"current_hour": "2025-01-23T12:00:00Z", "count": 7, "recent_timestamps": [1735689000, ...]}
```

### events

Immutable log of all blockchain events from Ponder indexers.

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY, -- Format: {chain_id}-{block_number}-{log_index}
    chain_id INTEGER NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    event_type TEXT NOT NULL,

    -- Common fields
    agent_id BIGINT,
    timestamp BIGINT NOT NULL, -- Block timestamp (Unix seconds)

    -- Identity Registry fields
    owner TEXT,
    token_uri TEXT,
    metadata_key TEXT,
    metadata_value TEXT,

    -- Reputation Registry fields
    client_address TEXT,
    feedback_index BIGINT,
    score INTEGER,
    tag1 TEXT,
    tag2 TEXT,
    file_uri TEXT,
    file_hash TEXT,

    -- Validation Registry fields
    validator_address TEXT,
    request_hash TEXT,
    response INTEGER,
    response_uri TEXT,
    response_hash TEXT,
    tag TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Convert to TimescaleDB hypertable for time-series optimization
SELECT create_hypertable('events', 'created_at', chunk_time_interval => INTERVAL '7 days');

-- Indexes for common query patterns
CREATE INDEX idx_events_chain_id_created_at ON events(chain_id, created_at DESC);
CREATE INDEX idx_events_agent_id ON events(agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_events_registry_type ON events(registry, event_type);
CREATE INDEX idx_events_client_address ON events(client_address) WHERE client_address IS NOT NULL;
CREATE INDEX idx_events_validator_address ON events(validator_address) WHERE validator_address IS NOT NULL;
CREATE INDEX idx_events_block_number ON events(chain_id, block_number);

-- Trigger function to notify new events via PostgreSQL NOTIFY
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
```

### checkpoints

Tracks last processed block per chain for Ponder indexers.

```sql
CREATE TABLE checkpoints (
    chain_id INTEGER PRIMARY KEY,
    last_block_number BIGINT NOT NULL,
    last_block_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### action_results

Audit trail of all action executions.

```sql
CREATE TABLE action_results (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    job_id TEXT NOT NULL, -- Redis job ID
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
```

### agent_mcp_tokens (Optional)

Stores authentication tokens for agent MCP servers.

```sql
CREATE TABLE agent_mcp_tokens (
    agent_id BIGINT PRIMARY KEY,
    token TEXT NOT NULL, -- Should be encrypted at rest
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TRIGGER update_agent_mcp_tokens_updated_at
    BEFORE UPDATE ON agent_mcp_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

## Materialized Views

### events_hourly

Continuous aggregate for hourly event statistics (TimescaleDB).

```sql
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
```

### action_metrics_hourly

Hourly action execution metrics.

```sql
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

-- Manual refresh (can be scheduled via cron or pg_cron)
REFRESH MATERIALIZED VIEW action_metrics_hourly;
```

## Retention Policies

### Events Retention

Optional: Archive old events to cold storage after 1 year.

```sql
-- Add retention policy (drops chunks older than 365 days)
SELECT add_retention_policy('events', INTERVAL '365 days');
```

### Action Results Retention

```sql
-- Delete action results older than 90 days
CREATE OR REPLACE FUNCTION cleanup_old_action_results()
RETURNS void AS $$
BEGIN
    DELETE FROM action_results
    WHERE executed_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;

-- Schedule via pg_cron (if extension available)
-- SELECT cron.schedule('cleanup-action-results', '0 2 * * *', 'SELECT cleanup_old_action_results()');
```

## Data Migration Strategy

### Applying Migrations

Using SQLx CLI:

```bash
# Create new migration
sqlx migrate add create_triggers_table

# Apply migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

### Migration Example

```sql
-- migrations/20250123_001_create_users_table.up.sql
CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- migrations/20250123_001_create_users_table.down.sql
DROP TABLE IF EXISTS users;
```

## Performance Optimization

### Query Optimization Tips

1. **Use indexes for filtering**:
   ```sql
   -- Good: Uses idx_triggers_chain_registry_enabled
   SELECT * FROM triggers
   WHERE chain_id = 84532 AND registry = 'reputation' AND enabled = true;
   ```

2. **Avoid SELECT * when possible**:
   ```sql
   -- Better: Select only needed columns
   SELECT id, name, chain_id FROM triggers WHERE user_id = 'user_123';
   ```

3. **Use EXPLAIN ANALYZE**:
   ```sql
   EXPLAIN ANALYZE
   SELECT * FROM events
   WHERE agent_id = 42 AND created_at > NOW() - INTERVAL '7 days';
   ```

### Connection Pooling

Configure SQLx connection pool in Rust:

```rust
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};

let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(3))
    .connect_with(
        PgConnectOptions::new()
            .host("localhost")
            .port(5432)
            .database("erc8004_backend")
            .username("postgres")
            .password("password")
    )
    .await?;
```

### Vacuum and Analyze

```sql
-- Regular maintenance (auto-vacuum should handle this)
VACUUM ANALYZE triggers;
VACUUM ANALYZE events;
VACUUM ANALYZE action_results;

-- Full vacuum (offline, reclaims disk space)
VACUUM FULL triggers;
```

## Backup and Recovery

### Backup Strategy

```bash
# Full database backup
pg_dump -Fc erc8004_backend > backup_$(date +%Y%m%d).dump

# Schema-only backup
pg_dump -s erc8004_backend > schema.sql

# Data-only backup
pg_dump -a erc8004_backend > data.sql
```

### Point-in-Time Recovery (PITR)

Enable WAL archiving in postgresql.conf:

```
wal_level = replica
archive_mode = on
archive_command = 'cp %p /var/lib/postgresql/wal_archive/%f'
```

### Restore

```bash
# Restore from dump
pg_restore -d erc8004_backend backup_20250123.dump

# Restore from SQL
psql erc8004_backend < backup.sql
```

## Security Considerations

### Row-Level Security (RLS)

Enable RLS for multi-tenant isolation:

```sql
-- Enable RLS on triggers table
ALTER TABLE triggers ENABLE ROW LEVEL SECURITY;

-- Policy: Users can only see their own triggers
CREATE POLICY triggers_user_isolation ON triggers
    FOR ALL
    USING (user_id = current_setting('app.current_user_id')::TEXT);
```

### Encryption at Rest

Use PostgreSQL encryption features or cloud provider encryption (AWS RDS encryption, etc.).

### Sensitive Data

- **Never store plaintext passwords**: Use bcrypt or argon2
- **Encrypt MCP tokens**: Use pgcrypto or application-level encryption
- **Audit access**: Enable PostgreSQL logging for sensitive tables

## Monitoring

### Useful Queries

**Active connections**:
```sql
SELECT count(*) FROM pg_stat_activity WHERE datname = 'erc8004_backend';
```

**Table sizes**:
```sql
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

**Index usage**:
```sql
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;
```

**Slow queries** (requires pg_stat_statements extension):
```sql
SELECT
    query,
    calls,
    total_exec_time,
    mean_exec_time,
    max_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 20;
```

## References

- **PostgreSQL Documentation**: https://www.postgresql.org/docs/15/
- **TimescaleDB Documentation**: https://docs.timescale.com/
- **SQLx Documentation**: https://github.com/launchbadge/sqlx
