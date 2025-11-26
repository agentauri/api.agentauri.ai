/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-argument */
/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Ponder Event Handlers
 *
 * Note: ESLint unsafe rules are disabled because Ponder's dynamic API
 * doesn't provide complete TypeScript type definitions for event handlers.
 * The `event` and `context` objects are strongly typed at runtime by Ponder.
 */
import { ponder } from "@ponder/core";
import type { Address, Hash, Hex } from "viem";

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Generate a unique event ID from chain, transaction, and log index
 */
function generateEventId(chainId: bigint, transactionHash: Hash, logIndex: number): string {
  return `${chainId}-${transactionHash}-${logIndex}`;
}

/**
 * Bytes32 to hex string converter
 */
function bytes32ToHex(bytes32: Hex): string {
  return bytes32;
}

// ============================================================================
// IDENTITY REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * AgentRegistered Event
 * Emitted when a new agent is registered in the Identity Registry
 */
ponder.on("IdentityRegistryEthereumSepolia:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, 11155111n);
});

ponder.on("IdentityRegistryBaseSepolia:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, 84532n);
});

ponder.on("IdentityRegistryLineaSepolia:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, 59141n);
});

ponder.on("IdentityRegistryPolygonAmoy:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, 80002n);
});

async function handleAgentRegistered(event: any, context: any, chainId: bigint): Promise<void> {
  const { Event } = context.db;

  const eventId = generateEventId(chainId, event.transaction.hash, event.log.logIndex);

  await Event.create({
    id: eventId,
    data: {
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry: "identity",
      eventType: "AgentRegistered",
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      owner: event.args.owner.toLowerCase() as Address,
      tokenUri: event.args.tokenURI,
    },
  });

  // Update checkpoint
  await context.db.Checkpoint.upsert({
    id: chainId,
    create: {
      chainId,
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
    update: {
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
  });
}

/**
 * MetadataUpdated Event
 * Emitted when agent metadata is updated
 */
ponder.on("IdentityRegistryEthereumSepolia:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, 11155111n);
});

ponder.on("IdentityRegistryBaseSepolia:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, 84532n);
});

ponder.on("IdentityRegistryLineaSepolia:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, 59141n);
});

ponder.on("IdentityRegistryPolygonAmoy:MetadataUpdated", async ({ event, context }) => {
  await handleMetadataUpdated(event, context, 80002n);
});

async function handleMetadataUpdated(event: any, context: any, chainId: bigint): Promise<void> {
  const { Event } = context.db;

  const eventId = generateEventId(chainId, event.transaction.hash, event.log.logIndex);

  await Event.create({
    id: eventId,
    data: {
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry: "identity",
      eventType: "MetadataUpdated",
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      metadataKey: event.args.key,
      metadataValue: event.args.value,
    },
  });

  // Update checkpoint
  await context.db.Checkpoint.upsert({
    id: chainId,
    create: {
      chainId,
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
    update: {
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
  });
}

// ============================================================================
// REPUTATION REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * FeedbackSubmitted Event
 * Emitted when feedback is submitted for an agent
 */
ponder.on("ReputationRegistryEthereumSepolia:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, 11155111n);
});

ponder.on("ReputationRegistryBaseSepolia:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, 84532n);
});

ponder.on("ReputationRegistryLineaSepolia:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, 59141n);
});

ponder.on("ReputationRegistryPolygonAmoy:FeedbackSubmitted", async ({ event, context }) => {
  await handleFeedbackSubmitted(event, context, 80002n);
});

async function handleFeedbackSubmitted(event: any, context: any, chainId: bigint): Promise<void> {
  const { Event } = context.db;

  const eventId = generateEventId(chainId, event.transaction.hash, event.log.logIndex);

  await Event.create({
    id: eventId,
    data: {
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry: "reputation",
      eventType: "FeedbackSubmitted",
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      clientAddress: event.args.client.toLowerCase() as Address,
      feedbackIndex: event.args.feedbackIndex,
      score: Number(event.args.score),
      tag1: event.args.tag1,
      tag2: event.args.tag2,
      fileUri: event.args.fileURI,
      fileHash: bytes32ToHex(event.args.fileHash),
    },
  });

  // Update checkpoint
  await context.db.Checkpoint.upsert({
    id: chainId,
    create: {
      chainId,
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
    update: {
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
  });
}

/**
 * ScoreUpdated Event
 * Emitted when an agent's reputation score is updated
 */
ponder.on("ReputationRegistryEthereumSepolia:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, 11155111n);
});

ponder.on("ReputationRegistryBaseSepolia:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, 84532n);
});

ponder.on("ReputationRegistryLineaSepolia:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, 59141n);
});

ponder.on("ReputationRegistryPolygonAmoy:ScoreUpdated", async ({ event, context }) => {
  await handleScoreUpdated(event, context, 80002n);
});

async function handleScoreUpdated(event: any, context: any, chainId: bigint): Promise<void> {
  const { Event } = context.db;

  const eventId = generateEventId(chainId, event.transaction.hash, event.log.logIndex);

  await Event.create({
    id: eventId,
    data: {
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry: "reputation",
      eventType: "ScoreUpdated",
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      score: Number(event.args.newScore),
      feedbackIndex: event.args.feedbackCount,
    },
  });

  // Update checkpoint
  await context.db.Checkpoint.upsert({
    id: chainId,
    create: {
      chainId,
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
    update: {
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
  });
}

// ============================================================================
// VALIDATION REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * ValidationPerformed Event
 * Emitted when a validation is performed for an agent
 */
ponder.on("ValidationRegistryEthereumSepolia:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, 11155111n);
});

ponder.on("ValidationRegistryBaseSepolia:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, 84532n);
});

ponder.on("ValidationRegistryLineaSepolia:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, 59141n);
});

ponder.on("ValidationRegistryPolygonAmoy:ValidationPerformed", async ({ event, context }) => {
  await handleValidationPerformed(event, context, 80002n);
});

async function handleValidationPerformed(event: any, context: any, chainId: bigint): Promise<void> {
  const { Event } = context.db;

  const eventId = generateEventId(chainId, event.transaction.hash, event.log.logIndex);

  await Event.create({
    id: eventId,
    data: {
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry: "validation",
      eventType: "ValidationPerformed",
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      validatorAddress: event.args.validator.toLowerCase() as Address,
      requestHash: bytes32ToHex(event.args.requestHash),
      response: Number(event.args.response),
      responseUri: event.args.responseURI,
      responseHash: bytes32ToHex(event.args.responseHash),
      tag: event.args.tag,
    },
  });

  // Update checkpoint
  await context.db.Checkpoint.upsert({
    id: chainId,
    create: {
      chainId,
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
    update: {
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
  });
}

/**
 * ValidationRequested Event
 * Emitted when a validation is requested for an agent
 */
ponder.on("ValidationRegistryEthereumSepolia:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, 11155111n);
});

ponder.on("ValidationRegistryBaseSepolia:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, 84532n);
});

ponder.on("ValidationRegistryLineaSepolia:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, 59141n);
});

ponder.on("ValidationRegistryPolygonAmoy:ValidationRequested", async ({ event, context }) => {
  await handleValidationRequested(event, context, 80002n);
});

async function handleValidationRequested(event: any, context: any, chainId: bigint): Promise<void> {
  const { Event } = context.db;

  const eventId = generateEventId(chainId, event.transaction.hash, event.log.logIndex);

  await Event.create({
    id: eventId,
    data: {
      chainId,
      blockNumber: event.block.number,
      blockHash: event.block.hash,
      transactionHash: event.transaction.hash,
      logIndex: event.log.logIndex,
      registry: "validation",
      eventType: "ValidationRequested",
      agentId: event.args.agentId,
      timestamp: event.args.timestamp,
      validatorAddress: event.args.validator.toLowerCase() as Address,
      requestHash: bytes32ToHex(event.args.requestHash),
    },
  });

  // Update checkpoint
  await context.db.Checkpoint.upsert({
    id: chainId,
    create: {
      chainId,
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
    update: {
      lastBlockNumber: event.block.number,
      lastBlockHash: event.block.hash,
    },
  });
}
