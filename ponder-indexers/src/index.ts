/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-argument */
/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Ponder Event Handlers for Ponder 0.7.x
 *
 * Note: ESLint unsafe rules are disabled because Ponder's dynamic API
 * doesn't provide complete TypeScript type definitions for event handlers.
 * The `event` and `context` objects are strongly typed at runtime by Ponder.
 */
import { ponder } from "@/generated";
import type { Address } from "viem";
import { Event, Checkpoint } from "../ponder.schema";
import { logEventProcessed, logEventError, logCheckpointUpdated } from "./logger";
import {
  CHAIN_IDS,
  REGISTRIES,
  generateEventId,
  bytes32ToHex,
} from "./helpers";

// Re-export for backwards compatibility
export { CHAIN_IDS, REGISTRIES, generateEventId, bytes32ToHex };

// ============================================================================
// IDENTITY REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * AgentRegistered Event
 * Emitted when a new agent is registered in the Identity Registry
 */
ponder.on("IdentityRegistryEthereumSepolia:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("IdentityRegistryBaseSepolia:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("IdentityRegistryLineaSepolia:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// PolygonAmoy handlers commented out (RPC not configured)
// ponder.on("IdentityRegistryPolygonAmoy:AgentRegistered", async ({ event, context }) => {
//   await handleAgentRegistered(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleAgentRegistered(event: any, context: any, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "AgentRegistered";

  try {
    const eventId = generateEventId(
      registry,
      chainId,
      event.transaction.hash,
      event.log.logIndex
    );

    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry,
      eventType,
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      owner: event.args.owner.toLowerCase() as Address,
      tokenUri: event.args.tokenURI,
    });

    // Update checkpoint
    await context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      })
      .onConflictDoUpdate({
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      });

    logEventProcessed(registry, eventType, chainId, event.block.number, event.args.agentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error; // Re-throw to let Ponder handle retries
  }
}

/**
 * MetadataUpdated Event
 * Emitted when agent metadata is updated
 */
ponder.on("IdentityRegistryEthereumSepolia:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("IdentityRegistryBaseSepolia:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("IdentityRegistryLineaSepolia:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("IdentityRegistryPolygonAmoy:MetadataUpdated", async ({ event, context }) => {
//   await handleMetadataUpdated(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleMetadataUpdated(event: any, context: any, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "MetadataUpdated";

  try {
    const eventId = generateEventId(
      registry,
      chainId,
      event.transaction.hash,
      event.log.logIndex
    );

    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry,
      eventType,
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      metadataKey: event.args.key,
      metadataValue: event.args.value,
    });

    // Update checkpoint
    await context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      })
      .onConflictDoUpdate({
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      });

    logEventProcessed(registry, eventType, chainId, event.block.number, event.args.agentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

// ============================================================================
// REPUTATION REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * FeedbackSubmitted Event
 * Emitted when feedback is submitted for an agent
 */
ponder.on("ReputationRegistryEthereumSepolia:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ReputationRegistryBaseSepolia:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ReputationRegistryLineaSepolia:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ReputationRegistryPolygonAmoy:FeedbackSubmitted", async ({ event, context }) => {
//   await handleFeedbackSubmitted(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleFeedbackSubmitted(event: any, context: any, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.REPUTATION;
  const eventType = "FeedbackSubmitted";

  try {
    const eventId = generateEventId(
      registry,
      chainId,
      event.transaction.hash,
      event.log.logIndex
    );

    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry,
      eventType,
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      clientAddress: event.args.client.toLowerCase() as Address,
      feedbackIndex: event.args.feedbackIndex,
      score: Number(event.args.score),
      tag1: event.args.tag1,
      tag2: event.args.tag2,
      fileUri: event.args.fileURI,
      fileHash: bytes32ToHex(event.args.fileHash),
    });

    // Update checkpoint
    await context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      })
      .onConflictDoUpdate({
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      });

    logEventProcessed(registry, eventType, chainId, event.block.number, event.args.agentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * ScoreUpdated Event
 * Emitted when an agent's reputation score is updated
 */
ponder.on("ReputationRegistryEthereumSepolia:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ReputationRegistryBaseSepolia:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ReputationRegistryLineaSepolia:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ReputationRegistryPolygonAmoy:ScoreUpdated", async ({ event, context }) => {
//   await handleScoreUpdated(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleScoreUpdated(event: any, context: any, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.REPUTATION;
  const eventType = "ScoreUpdated";

  try {
    const eventId = generateEventId(
      registry,
      chainId,
      event.transaction.hash,
      event.log.logIndex
    );

    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry,
      eventType,
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      score: Number(event.args.newScore),
      feedbackIndex: event.args.feedbackCount,
    });

    // Update checkpoint
    await context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      })
      .onConflictDoUpdate({
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      });

    logEventProcessed(registry, eventType, chainId, event.block.number, event.args.agentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

// ============================================================================
// VALIDATION REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * ValidationPerformed Event
 * Emitted when a validation is performed for an agent
 */
ponder.on("ValidationRegistryEthereumSepolia:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ValidationRegistryBaseSepolia:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ValidationRegistryLineaSepolia:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ValidationRegistryPolygonAmoy:ValidationPerformed", async ({ event, context }) => {
//   await handleValidationPerformed(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleValidationPerformed(event: any, context: any, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.VALIDATION;
  const eventType = "ValidationPerformed";

  try {
    const eventId = generateEventId(
      registry,
      chainId,
      event.transaction.hash,
      event.log.logIndex
    );

    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry,
      eventType,
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      validatorAddress: event.args.validator.toLowerCase() as Address,
      requestHash: bytes32ToHex(event.args.requestHash),
      response: Number(event.args.response),
      responseUri: event.args.responseURI,
      responseHash: bytes32ToHex(event.args.responseHash),
      tag: event.args.tag,
    });

    // Update checkpoint
    await context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      })
      .onConflictDoUpdate({
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      });

    logEventProcessed(registry, eventType, chainId, event.block.number, event.args.agentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * ValidationRequested Event
 * Emitted when a validation is requested for an agent
 */
ponder.on("ValidationRegistryEthereumSepolia:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ValidationRegistryBaseSepolia:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ValidationRegistryLineaSepolia:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ValidationRegistryPolygonAmoy:ValidationRequested", async ({ event, context }) => {
//   await handleValidationRequested(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleValidationRequested(event: any, context: any, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.VALIDATION;
  const eventType = "ValidationRequested";

  try {
    const eventId = generateEventId(
      registry,
      chainId,
      event.transaction.hash,
      event.log.logIndex
    );

    await context.db.insert(Event).values({
      id: eventId,
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry,
      eventType,
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      validatorAddress: event.args.validator.toLowerCase() as Address,
      requestHash: bytes32ToHex(event.args.requestHash),
    });

    // Update checkpoint
    await context.db
      .insert(Checkpoint)
      .values({
        id: chainId,
        chainId,
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      })
      .onConflictDoUpdate({
        lastBlockNumber: event.block.number,
        lastBlockHash: event.block.hash,
      });

    logEventProcessed(registry, eventType, chainId, event.block.number, event.args.agentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}
