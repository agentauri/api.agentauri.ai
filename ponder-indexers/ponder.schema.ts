import { createSchema } from "@ponder/core";

// ============================================================================
// PONDER SCHEMA DEFINITION
// ============================================================================
// This schema defines the tables that Ponder will create in PostgreSQL.
// It mirrors the existing database schema's events table structure.

export default createSchema((p) => ({
  // Events table - stores all blockchain events from ERC-8004 registries
  Event: p.createTable({
    // Primary key - composite of chain_id, transaction_hash, and log_index
    id: p.string(),

    // Chain and block information
    chainId: p.bigint(),
    blockNumber: p.bigint(),
    blockHash: p.string(),
    transactionHash: p.string(),
    logIndex: p.int(),

    // Registry and event type
    registry: p.string(), // 'identity', 'reputation', or 'validation'
    eventType: p.string(), // Event name from the contract

    // Common fields (may be null depending on event type)
    agentId: p.bigint().optional(),
    timestamp: p.bigint(),

    // Identity Registry specific fields
    owner: p.string().optional(),
    tokenUri: p.string().optional(),
    metadataKey: p.string().optional(),
    metadataValue: p.string().optional(),

    // Reputation Registry specific fields
    clientAddress: p.string().optional(),
    feedbackIndex: p.bigint().optional(),
    score: p.int().optional(),
    tag1: p.string().optional(),
    tag2: p.string().optional(),
    fileUri: p.string().optional(),
    fileHash: p.string().optional(),

    // Validation Registry specific fields
    validatorAddress: p.string().optional(),
    requestHash: p.string().optional(),
    response: p.int().optional(),
    responseUri: p.string().optional(),
    responseHash: p.string().optional(),
    tag: p.string().optional(),
  }),

  // Checkpoint table - tracks last processed block per chain
  Checkpoint: p.createTable({
    chainId: p.bigint(),
    lastBlockNumber: p.bigint(),
    lastBlockHash: p.string(),
  }),
}));
