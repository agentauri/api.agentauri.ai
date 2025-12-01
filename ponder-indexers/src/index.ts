/**
 * Ponder Event Handlers for Ponder 0.7.x
 *
 * Handles blockchain events from ERC-8004 registries (Identity, Reputation, Validation)
 * with comprehensive validation and type safety.
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
import {
  validateAndNormalizeAddress,
  validateAgentId,
  validateScore,
  validateUri,
  validateBytes32Hash,
  validateMetadataKey,
  validateMetadataValue,
  validateTag,
  validateFeedbackIndex,
} from "./validation";
import type {
  RegisteredEvent,
  MetadataSetEvent,
  UriUpdatedEvent,
  TransferEvent,
  NewFeedbackEvent,
  FeedbackRevokedEvent,
  ResponseAppendedEvent,
  ValidationRequestEvent,
  ValidationResponseEvent,
  PonderContext,
} from "./types";

// Re-export for backwards compatibility
export { CHAIN_IDS, REGISTRIES, generateEventId, bytes32ToHex };

// ============================================================================
// IDENTITY REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * Registered Event
 * Emitted when a new agent is registered in the Identity Registry
 */
ponder.on("IdentityRegistryEthereumSepolia:Registered", async ({ event, context }) => {
  await handleRegistered(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("IdentityRegistryBaseSepolia:Registered", async ({ event, context }) => {
  await handleRegistered(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("IdentityRegistryLineaSepolia:Registered", async ({ event, context }) => {
  await handleRegistered(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// PolygonAmoy handlers commented out (RPC not configured)
// ponder.on("IdentityRegistryPolygonAmoy:Registered", async ({ event, context }) => {
//   await handleRegistered(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleRegistered(event: RegisteredEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "Registered";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedOwner = validateAndNormalizeAddress(event.args.owner, "owner");
    const validatedTokenUri = validateUri(event.args.tokenURI, "tokenURI");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      owner: validatedOwner,
      tokenUri: validatedTokenUri,
    });

    // Update checkpoint (with race condition protection)
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error; // Re-throw to let Ponder handle retries
  }
}

/**
 * MetadataSet Event
 * Emitted when agent metadata is set/updated
 */
ponder.on("IdentityRegistryEthereumSepolia:MetadataSet", async ({ event, context }) => {
  await handleMetadataSet(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("IdentityRegistryBaseSepolia:MetadataSet", async ({ event, context }) => {
  await handleMetadataSet(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("IdentityRegistryLineaSepolia:MetadataSet", async ({ event, context }) => {
  await handleMetadataSet(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("IdentityRegistryPolygonAmoy:MetadataSet", async ({ event, context }) => {
//   await handleMetadataSet(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleMetadataSet(event: MetadataSetEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "MetadataSet";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedKey = validateMetadataKey(event.args.key);
    const validatedValue = validateMetadataValue(event.args.value);

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      metadataKey: validatedKey,
      metadataValue: validatedValue,
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * UriUpdated Event
 * Emitted when an agent's tokenURI is updated
 */
ponder.on("IdentityRegistryEthereumSepolia:UriUpdated", async ({ event, context }) => {
  await handleUriUpdated(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("IdentityRegistryBaseSepolia:UriUpdated", async ({ event, context }) => {
  await handleUriUpdated(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("IdentityRegistryLineaSepolia:UriUpdated", async ({ event, context }) => {
  await handleUriUpdated(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

async function handleUriUpdated(event: UriUpdatedEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "UriUpdated";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedNewUri = validateUri(event.args.newUri, "newUri");
    const validatedUpdatedBy = validateAndNormalizeAddress(event.args.updatedBy, "updatedBy");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      tokenUri: validatedNewUri,
      owner: validatedUpdatedBy, // Store updatedBy in owner field (schema reuse)
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * Transfer Event (ERC721)
 * Emitted when agent ownership is transferred
 */
ponder.on("IdentityRegistryEthereumSepolia:Transfer", async ({ event, context }) => {
  await handleTransfer(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("IdentityRegistryBaseSepolia:Transfer", async ({ event, context }) => {
  await handleTransfer(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("IdentityRegistryLineaSepolia:Transfer", async ({ event, context }) => {
  await handleTransfer(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

async function handleTransfer(event: TransferEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.IDENTITY;
  const eventType = "Transfer";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.tokenId);
    const validatedTo = validateAndNormalizeAddress(event.args.to, "to");
    const validatedFrom = validateAndNormalizeAddress(event.args.from, "from");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      owner: validatedTo, // New owner
      clientAddress: validatedFrom, // Previous owner (schema reuse)
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
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
 * NewFeedback Event
 * Emitted when feedback is submitted for an agent
 */
ponder.on("ReputationRegistryEthereumSepolia:NewFeedback", async ({ event, context }) => {
  await handleNewFeedback(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ReputationRegistryBaseSepolia:NewFeedback", async ({ event, context }) => {
  await handleNewFeedback(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ReputationRegistryLineaSepolia:NewFeedback", async ({ event, context }) => {
  await handleNewFeedback(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ReputationRegistryPolygonAmoy:NewFeedback", async ({ event, context }) => {
//   await handleNewFeedback(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleNewFeedback(event: NewFeedbackEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.REPUTATION;
  const eventType = "NewFeedback";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedClientAddress = validateAndNormalizeAddress(event.args.clientAddress, "clientAddress");
    const validatedScore = validateScore(event.args.score);
    const validatedTag1 = validateTag(event.args.tag1, "tag1");
    const validatedTag2 = validateTag(event.args.tag2, "tag2");
    const validatedFeedbackUri = validateUri(event.args.feedbackUri, "feedbackUri");
    const validatedFeedbackHash = validateBytes32Hash(event.args.feedbackHash, "feedbackHash");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      clientAddress: validatedClientAddress,
      feedbackIndex: null, // NewFeedback doesn't emit feedbackIndex (contract-assigned)
      score: validatedScore,
      tag1: validatedTag1,
      tag2: validatedTag2,
      fileUri: validatedFeedbackUri,
      fileHash: validatedFeedbackHash,
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * FeedbackRevoked Event
 * Emitted when feedback is revoked by the client
 */
ponder.on("ReputationRegistryEthereumSepolia:FeedbackRevoked", async ({ event, context }) => {
  await handleFeedbackRevoked(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ReputationRegistryBaseSepolia:FeedbackRevoked", async ({ event, context }) => {
  await handleFeedbackRevoked(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ReputationRegistryLineaSepolia:FeedbackRevoked", async ({ event, context }) => {
  await handleFeedbackRevoked(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

async function handleFeedbackRevoked(event: FeedbackRevokedEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.REPUTATION;
  const eventType = "FeedbackRevoked";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedClientAddress = validateAndNormalizeAddress(event.args.clientAddress, "clientAddress");
    const validatedFeedbackIndex = validateFeedbackIndex(event.args.feedbackIndex);

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      clientAddress: validatedClientAddress,
      feedbackIndex: validatedFeedbackIndex,
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * ResponseAppended Event
 * Emitted when a response is appended to existing feedback
 */
ponder.on("ReputationRegistryEthereumSepolia:ResponseAppended", async ({ event, context }) => {
  await handleResponseAppended(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ReputationRegistryBaseSepolia:ResponseAppended", async ({ event, context }) => {
  await handleResponseAppended(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ReputationRegistryLineaSepolia:ResponseAppended", async ({ event, context }) => {
  await handleResponseAppended(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

async function handleResponseAppended(event: ResponseAppendedEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.REPUTATION;
  const eventType = "ResponseAppended";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedClientAddress = validateAndNormalizeAddress(event.args.clientAddress, "clientAddress");
    const validatedFeedbackIndex = validateFeedbackIndex(event.args.feedbackIndex);
    const validatedResponder = validateAndNormalizeAddress(event.args.responder, "responder");
    const validatedResponseUri = validateUri(event.args.responseUri, "responseUri");
    const validatedResponseHash = validateBytes32Hash(event.args.responseHash, "responseHash");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      clientAddress: validatedClientAddress,
      feedbackIndex: validatedFeedbackIndex,
      validatorAddress: validatedResponder, // Reuse validatorAddress for responder (schema reuse)
      responseUri: validatedResponseUri,
      responseHash: validatedResponseHash,
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

// NOTE: ScoreUpdated event removed - not emitted by deployed contract

// ============================================================================
// VALIDATION REGISTRY EVENT HANDLERS
// ============================================================================

/**
 * ValidationResponse Event
 * Emitted when a validation response is submitted
 */
ponder.on("ValidationRegistryEthereumSepolia:ValidationResponse", async ({ event, context }) => {
  await handleValidationResponse(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ValidationRegistryBaseSepolia:ValidationResponse", async ({ event, context }) => {
  await handleValidationResponse(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ValidationRegistryLineaSepolia:ValidationResponse", async ({ event, context }) => {
  await handleValidationResponse(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ValidationRegistryPolygonAmoy:ValidationResponse", async ({ event, context }) => {
//   await handleValidationResponse(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleValidationResponse(event: ValidationResponseEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.VALIDATION;
  const eventType = "ValidationResponse";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedValidatorAddress = validateAndNormalizeAddress(event.args.validatorAddress, "validatorAddress");
    const validatedRequestHash = validateBytes32Hash(event.args.requestHash, "requestHash");
    const validatedResponseUri = validateUri(event.args.responseUri, "responseUri");
    const validatedResponseHash = validateBytes32Hash(event.args.responseHash, "responseHash");
    const validatedTag = validateTag(event.args.tag, "tag");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      validatorAddress: validatedValidatorAddress,
      requestHash: validatedRequestHash,
      response: Number(event.args.response),
      responseUri: validatedResponseUri,
      responseHash: validatedResponseHash,
      tag: validatedTag,
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}

/**
 * ValidationRequest Event
 * Emitted when a validation is requested for an agent
 */
ponder.on("ValidationRegistryEthereumSepolia:ValidationRequest", async ({ event, context }) => {
  await handleValidationRequest(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

ponder.on("ValidationRegistryBaseSepolia:ValidationRequest", async ({ event, context }) => {
  await handleValidationRequest(event, context, CHAIN_IDS.BASE_SEPOLIA);
});

ponder.on("ValidationRegistryLineaSepolia:ValidationRequest", async ({ event, context }) => {
  await handleValidationRequest(event, context, CHAIN_IDS.LINEA_SEPOLIA);
});

// ponder.on("ValidationRegistryPolygonAmoy:ValidationRequest", async ({ event, context }) => {
//   await handleValidationRequest(event, context, CHAIN_IDS.POLYGON_AMOY);
// });

async function handleValidationRequest(event: ValidationRequestEvent, context: PonderContext, chainId: bigint): Promise<void> {
  const registry = REGISTRIES.VALIDATION;
  const eventType = "ValidationRequest";

  try {
    // Validate inputs
    const validatedAgentId = validateAgentId(event.args.agentId);
    const validatedValidatorAddress = validateAndNormalizeAddress(event.args.validatorAddress, "validatorAddress");
    const validatedRequestHash = validateBytes32Hash(event.args.requestHash, "requestHash");
    const validatedRequestUri = validateUri(event.args.requestUri, "requestUri");

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
      agentId: validatedAgentId,
      timestamp: event.block.timestamp,
      validatorAddress: validatedValidatorAddress,
      requestHash: validatedRequestHash,
      requestUri: validatedRequestUri,
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

    logEventProcessed(registry, eventType, chainId, event.block.number, validatedAgentId);
    logCheckpointUpdated(chainId, event.block.number);
  } catch (error) {
    logEventError(registry, eventType, chainId, error as Error);
    throw error;
  }
}
