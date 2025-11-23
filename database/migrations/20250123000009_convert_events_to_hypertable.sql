-- Migration 9: Convert events to TimescaleDB Hypertable
-- Description: Converts events table to TimescaleDB hypertable and creates indexes
-- Created: 2025-01-23
-- Note: Indexes must be created AFTER hypertable conversion for optimal performance

-- Convert to TimescaleDB hypertable for time-series optimization
-- Partitions data into 7-day chunks based on created_at timestamp
SELECT create_hypertable('events', 'created_at', chunk_time_interval => INTERVAL '7 days');

-- Indexes for common query patterns
-- Note: TimescaleDB automatically creates indexes on the partitioning column

-- Index for filtering by chain and time (most common query pattern)
CREATE INDEX idx_events_chain_id_created_at ON events(chain_id, created_at DESC);

-- Index for agent-specific queries (partial index excludes NULL values)
CREATE INDEX idx_events_agent_id ON events(agent_id) WHERE agent_id IS NOT NULL;

-- Index for filtering by registry and event type
CREATE INDEX idx_events_registry_type ON events(registry, event_type);

-- Index for client address lookups (partial index for reputation events)
CREATE INDEX idx_events_client_address ON events(client_address) WHERE client_address IS NOT NULL;

-- Index for validator address lookups (partial index for validation events)
CREATE INDEX idx_events_validator_address ON events(validator_address) WHERE validator_address IS NOT NULL;

-- Index for block number queries (useful for syncing and debugging)
CREATE INDEX idx_events_block_number ON events(chain_id, block_number);
