-- Migration 10: Create checkpoints Table
-- Description: Tracks last processed block per chain for Ponder indexers
-- Created: 2025-01-23

CREATE TABLE checkpoints (
    chain_id INTEGER PRIMARY KEY,
    last_block_number BIGINT NOT NULL,
    last_block_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Note: No additional indexes needed as chain_id is the primary key
-- This table will have very few rows (one per chain) and is updated frequently
