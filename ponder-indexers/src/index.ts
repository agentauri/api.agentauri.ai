/**
 * Ponder Event Handlers for Ponder 0.7.x
 *
 * Handles blockchain events from ERC-8004 registries (Identity, Reputation, Validation)
 * with comprehensive validation and type safety.
 */
import { ponder } from "@/generated";
import {
  CHAIN_IDS,
  REGISTRIES,
  processEvent,
  bytes32ToHex,
  generateEventId,
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

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// Uncomment when contracts are deployed and PONDER_CONFIG addresses are set
// ponder.on("IdentityRegistryBaseSepolia:Registered", async ({ event, context }) => {
//   await handleRegistered(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("IdentityRegistryLineaSepolia:Registered", async ({ event, context }) => {
//   await handleRegistered(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleRegistered(event: RegisteredEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.IDENTITY,
      eventType: "Registered",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        owner: validateAndNormalizeAddress(event.args.owner, "owner"),
        agentUri: validateUri(event.args.agentURI, "agentURI"),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] Registered event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

/**
 * MetadataSet Event
 * Emitted when agent metadata is set/updated
 */
ponder.on("IdentityRegistryEthereumSepolia:MetadataSet", async ({ event, context }) => {
  await handleMetadataSet(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("IdentityRegistryBaseSepolia:MetadataSet", async ({ event, context }) => {
//   await handleMetadataSet(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("IdentityRegistryLineaSepolia:MetadataSet", async ({ event, context }) => {
//   await handleMetadataSet(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleMetadataSet(event: MetadataSetEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.IDENTITY,
      eventType: "MetadataSet",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        metadataKey: validateMetadataKey(event.args.metadataKey),
        metadataValue: validateMetadataValue(event.args.metadataValue),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] MetadataSet event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

/**
 * URIUpdated Event
 * Emitted when an agent's agentURI is updated
 * ERC-8004 v1.0: Event renamed from UriUpdated to URIUpdated, newUri to newURI
 */
ponder.on("IdentityRegistryEthereumSepolia:URIUpdated", async ({ event, context }) => {
  await handleUriUpdated(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("IdentityRegistryBaseSepolia:URIUpdated", async ({ event, context }) => {
//   await handleUriUpdated(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("IdentityRegistryLineaSepolia:URIUpdated", async ({ event, context }) => {
//   await handleUriUpdated(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleUriUpdated(event: UriUpdatedEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.IDENTITY,
      eventType: "URIUpdated",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        agentUri: validateUri(event.args.newURI, "newURI"),
        owner: validateAndNormalizeAddress(event.args.updatedBy, "updatedBy"), // Store updatedBy in owner field (schema reuse)
      },
    });
  } catch (error) {
    console.warn(`[SKIP] URIUpdated event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

/**
 * Transfer Event (ERC721)
 * Emitted when agent ownership is transferred
 */
ponder.on("IdentityRegistryEthereumSepolia:Transfer", async ({ event, context }) => {
  await handleTransfer(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("IdentityRegistryBaseSepolia:Transfer", async ({ event, context }) => {
//   await handleTransfer(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("IdentityRegistryLineaSepolia:Transfer", async ({ event, context }) => {
//   await handleTransfer(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleTransfer(event: TransferEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.tokenId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.IDENTITY,
      eventType: "Transfer",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        owner: validateAndNormalizeAddress(event.args.to, "to"), // New owner
        clientAddress: validateAndNormalizeAddress(event.args.from, "from"), // Previous owner (schema reuse)
      },
    });
  } catch (error) {
    console.warn(`[SKIP] Transfer event validation failed:`, {
      chainId: chainId.toString(),
      tokenId: event.args.tokenId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
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

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("ReputationRegistryBaseSepolia:NewFeedback", async ({ event, context }) => {
//   await handleNewFeedback(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("ReputationRegistryLineaSepolia:NewFeedback", async ({ event, context }) => {
//   await handleNewFeedback(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleNewFeedback(event: NewFeedbackEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.REPUTATION,
      eventType: "NewFeedback",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        clientAddress: validateAndNormalizeAddress(event.args.clientAddress, "clientAddress"),
        feedbackIndex: validateFeedbackIndex(event.args.feedbackIndex),
        score: validateScore(event.args.score),
        tag1: validateTag(event.args.tag1, "tag1"),
        tag2: validateTag(event.args.tag2, "tag2"),
        endpoint: event.args.endpoint || "",
        fileUri: validateUri(event.args.feedbackURI, "feedbackURI"),
        fileHash: validateBytes32Hash(event.args.feedbackHash, "feedbackHash"),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] NewFeedback event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

/**
 * FeedbackRevoked Event
 * Emitted when feedback is revoked by the client
 */
ponder.on("ReputationRegistryEthereumSepolia:FeedbackRevoked", async ({ event, context }) => {
  await handleFeedbackRevoked(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("ReputationRegistryBaseSepolia:FeedbackRevoked", async ({ event, context }) => {
//   await handleFeedbackRevoked(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("ReputationRegistryLineaSepolia:FeedbackRevoked", async ({ event, context }) => {
//   await handleFeedbackRevoked(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleFeedbackRevoked(event: FeedbackRevokedEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.REPUTATION,
      eventType: "FeedbackRevoked",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        clientAddress: validateAndNormalizeAddress(event.args.clientAddress, "clientAddress"),
        feedbackIndex: validateFeedbackIndex(event.args.feedbackIndex),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] FeedbackRevoked event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

/**
 * ResponseAppended Event
 * Emitted when a response is appended to existing feedback
 */
ponder.on("ReputationRegistryEthereumSepolia:ResponseAppended", async ({ event, context }) => {
  await handleResponseAppended(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
});

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("ReputationRegistryBaseSepolia:ResponseAppended", async ({ event, context }) => {
//   await handleResponseAppended(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("ReputationRegistryLineaSepolia:ResponseAppended", async ({ event, context }) => {
//   await handleResponseAppended(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleResponseAppended(event: ResponseAppendedEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.REPUTATION,
      eventType: "ResponseAppended",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        clientAddress: validateAndNormalizeAddress(event.args.clientAddress, "clientAddress"),
        feedbackIndex: validateFeedbackIndex(event.args.feedbackIndex),
        validatorAddress: validateAndNormalizeAddress(event.args.responder, "responder"), // Reuse validatorAddress for responder (schema reuse)
        responseUri: validateUri(event.args.responseURI, "responseURI"),
        responseHash: validateBytes32Hash(event.args.responseHash, "responseHash"),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] ResponseAppended event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
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
// NOTE: Validation Registry not yet deployed on Ethereum Sepolia - handler commented out
// Uncomment when ValidationRegistry is deployed and ETHEREUM_SEPOLIA_VALIDATION_ADDRESS is set
// ponder.on("ValidationRegistryEthereumSepolia:ValidationResponse", async ({ event, context }) => {
//   await handleValidationResponse(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
// });

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("ValidationRegistryBaseSepolia:ValidationResponse", async ({ event, context }) => {
//   await handleValidationResponse(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("ValidationRegistryLineaSepolia:ValidationResponse", async ({ event, context }) => {
//   await handleValidationResponse(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleValidationResponse(event: ValidationResponseEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.VALIDATION,
      eventType: "ValidationResponse",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        validatorAddress: validateAndNormalizeAddress(event.args.validatorAddress, "validatorAddress"),
        requestHash: validateBytes32Hash(event.args.requestHash, "requestHash"),
        response: Number(event.args.response),
        responseUri: validateUri(event.args.responseURI, "responseURI"),
        responseHash: validateBytes32Hash(event.args.responseHash, "responseHash"),
        tag: validateTag(event.args.tag, "tag"),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] ValidationResponse event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

/**
 * ValidationRequest Event
 * Emitted when a validation is requested for an agent
 */
// NOTE: Validation Registry not yet deployed on Ethereum Sepolia - handler commented out
// Uncomment when ValidationRegistry is deployed and ETHEREUM_SEPOLIA_VALIDATION_ADDRESS is set
// ponder.on("ValidationRegistryEthereumSepolia:ValidationRequest", async ({ event, context }) => {
//   await handleValidationRequest(event, context, CHAIN_IDS.ETHEREUM_SEPOLIA);
// });

// NOTE: Base Sepolia and Linea Sepolia handlers disabled until v1.0 contracts are deployed
// ponder.on("ValidationRegistryBaseSepolia:ValidationRequest", async ({ event, context }) => {
//   await handleValidationRequest(event, context, CHAIN_IDS.BASE_SEPOLIA);
// });
// ponder.on("ValidationRegistryLineaSepolia:ValidationRequest", async ({ event, context }) => {
//   await handleValidationRequest(event, context, CHAIN_IDS.LINEA_SEPOLIA);
// });

async function handleValidationRequest(event: ValidationRequestEvent, context: PonderContext, chainId: bigint): Promise<void> {
  try {
    const validatedAgentId = validateAgentId(event.args.agentId);

    await processEvent(context, event.block, event.transaction, event.log, {
      registry: REGISTRIES.VALIDATION,
      eventType: "ValidationRequest",
      chainId,
      agentId: validatedAgentId,
      eventValues: {
        validatorAddress: validateAndNormalizeAddress(event.args.validatorAddress, "validatorAddress"),
        requestHash: validateBytes32Hash(event.args.requestHash, "requestHash"),
        requestUri: validateUri(event.args.requestURI, "requestURI"),
      },
    });
  } catch (error) {
    console.warn(`[SKIP] ValidationRequest event validation failed:`, {
      chainId: chainId.toString(),
      agentId: event.args.agentId.toString(),
      error: error instanceof Error ? error.message : String(error),
    });
  }
}
