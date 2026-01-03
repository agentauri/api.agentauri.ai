/**
 * TypeScript Type Definitions for Ponder Event Handlers
 *
 * These types provide compile-time safety for event arguments and context,
 * replacing the previous 'any' types that disabled type checking.
 *
 * Based on Ponder 0.7.x event handler API and Viem types.
 */

import type { Address, Hex } from "viem";

// ============================================================================
// COMMON TYPES
// ============================================================================

/**
 * Blockchain block information
 */
export interface BlockInfo {
  number: bigint;
  hash: string;
  timestamp: bigint;
}

/**
 * Transaction information
 */
export interface TransactionInfo {
  hash: string;
}

/**
 * Log information
 */
export interface LogInfo {
  logIndex: number;
}

// ============================================================================
// IDENTITY REGISTRY EVENT TYPES
// ============================================================================

/**
 * Registered event arguments
 * Emitted when a new agent is registered
 */
export interface RegisteredEventArgs {
  agentId: bigint;
  tokenURI: string;
  owner: Address;
}

export interface RegisteredEvent {
  args: RegisteredEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

/**
 * MetadataSet event arguments
 * Emitted when agent metadata is updated
 */
export interface MetadataSetEventArgs {
  agentId: bigint;
  key: string;
  value: Uint8Array; // bytes in Solidity
}

export interface MetadataSetEvent {
  args: MetadataSetEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

/**
 * UriUpdated event arguments
 * Emitted when agent's tokenURI is updated
 */
export interface UriUpdatedEventArgs {
  agentId: bigint;
  newUri: string;
  updatedBy: Address;
}

export interface UriUpdatedEvent {
  args: UriUpdatedEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

/**
 * Transfer event arguments (ERC721)
 * Emitted when agent ownership is transferred
 */
export interface TransferEventArgs {
  from: Address;
  to: Address;
  tokenId: bigint;
}

export interface TransferEvent {
  args: TransferEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

// ============================================================================
// REPUTATION REGISTRY EVENT TYPES
// ============================================================================

/**
 * NewFeedback event arguments
 * Emitted when feedback is submitted for an agent
 */
export interface NewFeedbackEventArgs {
  agentId: bigint;
  clientAddress: Address;
  score: bigint; // uint8 in Solidity
  tag1: Hex; // bytes32
  tag2: Hex; // bytes32
  feedbackUri: string;
  feedbackHash: Hex; // bytes32
}

export interface NewFeedbackEvent {
  args: NewFeedbackEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

/**
 * FeedbackRevoked event arguments
 * Emitted when feedback is revoked by the client
 */
export interface FeedbackRevokedEventArgs {
  agentId: bigint;
  clientAddress: Address;
  feedbackIndex: bigint; // uint64 in Solidity
}

export interface FeedbackRevokedEvent {
  args: FeedbackRevokedEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

/**
 * ResponseAppended event arguments
 * Emitted when a response is appended to existing feedback
 */
export interface ResponseAppendedEventArgs {
  agentId: bigint;
  clientAddress: Address;
  feedbackIndex: bigint; // uint64 in Solidity
  responder: Address;
  responseUri: string;
  responseHash: Hex; // bytes32
}

export interface ResponseAppendedEvent {
  args: ResponseAppendedEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

// ============================================================================
// VALIDATION REGISTRY EVENT TYPES
// ============================================================================

/**
 * ValidationRequest event arguments
 * Emitted when a validation is requested
 */
export interface ValidationRequestEventArgs {
  validatorAddress: Address;
  agentId: bigint;
  requestHash: Hex; // bytes32
  requestUri: string;
}

export interface ValidationRequestEvent {
  args: ValidationRequestEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

/**
 * ValidationResponse event arguments
 * Emitted when a validation response is submitted
 */
export interface ValidationResponseEventArgs {
  validatorAddress: Address;
  agentId: bigint;
  requestHash: Hex; // bytes32
  response: bigint; // uint256 in Solidity
  responseUri: string;
  responseHash: Hex; // bytes32
  tag: Hex; // bytes32
}

export interface ValidationResponseEvent {
  args: ValidationResponseEventArgs;
  block: BlockInfo;
  transaction: TransactionInfo;
  log: LogInfo;
}

// ============================================================================
// PONDER CONTEXT TYPE
// ============================================================================

/**
 * Ponder database context
 *
 * Note: This is a simplified type. Ponder's actual context type is more
 * complex and includes additional methods. This type provides sufficient
 * type safety for our event handlers.
 */
export interface PonderContext {
  db: {
    insert: (table: unknown) => {
      values: (data: Record<string, unknown>) => {
        onConflictDoUpdate?: (update: Record<string, unknown>) => Promise<void>;
      } & Promise<void>;
    };
  };
}

// ============================================================================
// HELPER TYPE UNIONS
// ============================================================================

/**
 * Union of all event types for generic handlers
 */
export type AnyEvent =
  | RegisteredEvent
  | MetadataSetEvent
  | UriUpdatedEvent
  | TransferEvent
  | NewFeedbackEvent
  | FeedbackRevokedEvent
  | ResponseAppendedEvent
  | ValidationRequestEvent
  | ValidationResponseEvent;
