/**
 * Helper Functions and Constants for Ponder Event Handlers
 *
 * This file contains pure functions and constants that are testable
 * independently of the Ponder runtime.
 */
import type { Hash, Hex } from "viem";
import { Event, Checkpoint } from "../ponder.schema";
import { logEventProcessed, logEventError, logCheckpointUpdated } from "./logger";
import type { PonderContext, BlockInfo, TransactionInfo, LogInfo } from "./types";
import { enqueueFailedEvent, getDeadLetterQueue } from "./database";

// ============================================================================
// CONSTANTS
// ============================================================================

/**
 * Chain IDs for supported networks
 * Extracted from magic numbers for maintainability
 */
export const CHAIN_IDS = {
  ETHEREUM_SEPOLIA: 11155111n,
  BASE_SEPOLIA: 84532n,
  LINEA_SEPOLIA: 59141n,
  POLYGON_AMOY: 80002n,
} as const;

/**
 * Registry types for event categorization
 */
export const REGISTRIES = {
  IDENTITY: "identity",
  REPUTATION: "reputation",
  VALIDATION: "validation",
} as const;

export type Registry = typeof REGISTRIES[keyof typeof REGISTRIES];

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Generate a unique event ID from registry, chain, transaction, and log index
 * Including registry prevents collisions across different event types
 */
export function generateEventId(
  registry: Registry,
  chainId: bigint,
  transactionHash: Hash,
  logIndex: number
): string {
  return `${registry}-${chainId}-${transactionHash}-${logIndex}`;
}

/**
 * Bytes32 to hex string converter
 * Returns the hex string as-is since it's already in the correct format
 */
export function bytes32ToHex(bytes32: Hex): string {
  return bytes32;
}

// ============================================================================
// EVENT PROCESSING HELPER
// ============================================================================

/**
 * Event data for processEvent helper
 */
export interface EventData {
  registry: Registry;
  eventType: string;
  chainId: bigint;
  agentId: bigint;
  /** Additional event-specific fields */
  eventValues: Record<string, unknown>;
}

/**
 * Centralized event processing helper that handles:
 * - Event ID generation
 * - Database insert for Event table
 * - Checkpoint update
 * - Success/error logging
 * - Error re-throw for Ponder retry handling
 *
 * This reduces code duplication across all 9 event handlers.
 */
export async function processEvent(
  context: PonderContext,
  block: BlockInfo,
  transaction: TransactionInfo,
  log: LogInfo,
  data: EventData
): Promise<void> {
  const { registry, eventType, chainId, agentId, eventValues } = data;

  try {
    const eventId = generateEventId(registry, chainId, transaction.hash as Hash, log.logIndex);

    // Insert event
    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: block.number,
      blockHash: block.hash,
      transactionHash: transaction.hash,
      logIndex: log.logIndex,
      registry,
      eventType,
      agentId,
      timestamp: block.timestamp,
      ...eventValues,
    });

    // Update checkpoint (with race condition protection)
    const checkpointInsert = context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: block.number,
        lastBlockHash: block.hash,
      });

    if (checkpointInsert.onConflictDoUpdate) {
      await checkpointInsert.onConflictDoUpdate({
        lastBlockNumber: block.number,
        lastBlockHash: block.hash,
      });
    } else {
      await checkpointInsert;
    }

    logEventProcessed(registry, eventType, chainId, block.number, agentId);
    logCheckpointUpdated(chainId, block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);

    // Enqueue to DLQ if available, otherwise re-throw for Ponder retry
    const dlq = getDeadLetterQueue();
    if (dlq) {
      const eventId = generateEventId(registry, chainId, transaction.hash as Hash, log.logIndex);
      await enqueueFailedEvent(
        eventId,
        {
          registry,
          eventType,
          chainId: chainId.toString(),
          agentId: agentId.toString(),
          blockNumber: block.number.toString(),
          transactionHash: transaction.hash,
          logIndex: log.logIndex,
          timestamp: block.timestamp.toString(),
          eventValues,
        },
        error as Error
      );
      // Don't re-throw - let indexer continue with next event
      // DLQ will handle retry with exponential backoff
    } else {
      // Fallback: re-throw to let Ponder handle retries
      throw error;
    }
  }
}
