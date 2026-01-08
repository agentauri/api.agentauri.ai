// Ponder Schema Definition for Ponder 0.7.x
// Uses onchainTable from @ponder/core

import { onchainTable } from "@ponder/core";

// Events table - stores all blockchain events from ERC-8004 registries
export const Event = onchainTable("Event", (t) => ({
  // Primary key - composite of chain_id, transaction_hash, and log_index
  id: t.text().primaryKey(),

  // Chain and block information
  chainId: t.bigint().notNull(),
  blockNumber: t.bigint().notNull(),
  blockHash: t.text().notNull(),
  transactionHash: t.text().notNull(),
  logIndex: t.integer().notNull(),

  // Registry and event type
  registry: t.text().notNull(), // 'identity', 'reputation', or 'validation'
  eventType: t.text().notNull(), // Event name from the contract

  // Common fields (may be null depending on event type)
  agentId: t.bigint(),
  timestamp: t.bigint().notNull(),

  // Identity Registry specific fields
  owner: t.text(),
  agentUri: t.text(),
  metadataKey: t.text(),
  metadataValue: t.text(),

  // Reputation Registry specific fields
  clientAddress: t.text(),
  feedbackIndex: t.bigint(),
  score: t.integer(),
  tag1: t.text(),
  tag2: t.text(),
  fileUri: t.text(),
  fileHash: t.text(),
  endpoint: t.text(),

  // Validation Registry specific fields
  validatorAddress: t.text(),
  requestHash: t.text(),
  response: t.integer(),
  responseUri: t.text(),
  responseHash: t.text(),
  tag: t.text(),
}));

// Checkpoint table - tracks last processed block per chain
export const Checkpoint = onchainTable("Checkpoint", (t) => ({
  id: t.bigint().primaryKey(), // chainId as primary key
  chainId: t.bigint().notNull(),
  lastBlockNumber: t.bigint().notNull(),
  lastBlockHash: t.text().notNull(),
}));
