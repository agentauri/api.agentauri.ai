/**
 * Helper Functions and Constants for Ponder Event Handlers
 *
 * This file contains pure functions and constants that are testable
 * independently of the Ponder runtime.
 */
import type { Hash, Hex } from "viem";

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
