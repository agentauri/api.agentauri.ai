/**
 * Input Validation Utilities for Ponder Event Handlers
 *
 * Provides validation functions for blockchain data to ensure data integrity
 * and prevent runtime errors from malformed event parameters.
 *
 * Security Benefits:
 * - Prevents crashes from malformed addresses
 * - Validates numeric ranges (scores, IDs)
 * - Protects against SSRF via URI validation
 * - Enforces length limits to prevent storage abuse
 * - Validates hash formats
 */

import { isAddress, type Address, type Hex } from "viem";
import { eventLogger } from "./logger";

// ============================================================================
// ADDRESS VALIDATION
// ============================================================================

/**
 * Validate and normalize an Ethereum address
 *
 * Security: Prevents database insertion of invalid addresses that could
 * cause constraint violations or downstream processing errors.
 *
 * @param addr - Address to validate (from blockchain event)
 * @param fieldName - Field name for error messages (e.g., "owner", "clientAddress")
 * @returns Normalized lowercase address
 * @throws Error if address is invalid
 */
export function validateAndNormalizeAddress(
  addr: unknown,
  fieldName: string = "address"
): Address {
  // Type check
  if (!addr || typeof addr !== "string") {
    throw new Error(
      `Invalid ${fieldName} type: expected string, got ${typeof addr}`
    );
  }

  // Normalize to lowercase
  const normalized = addr.toLowerCase();

  // Validate format using viem's isAddress (checksum-aware)
  if (!isAddress(normalized)) {
    eventLogger.error(
      { fieldName, invalidAddress: addr },
      `Invalid Ethereum address format: ${fieldName}`
    );
    throw new Error(
      `Invalid ${fieldName} format: must be 40-character hex string with 0x prefix`
    );
  }

  // isAddress is a type guard, so normalized is now Address
  return normalized;
}

// ============================================================================
// NUMERIC VALIDATION
// ============================================================================

/**
 * Validate reputation score
 *
 * Security: Ensures scores are within expected range (0-100) to prevent
 * invalid reputation calculations and trigger logic errors.
 *
 * Note: ERC-8004 defines scores as uint8 (0-255), but business logic
 * expects 0-100. This function clamps out-of-range values.
 *
 * @param score - Score value from NewFeedback event (bigint or number)
 * @returns Validated score (clamped to 0-100)
 * @throws Error if score is not a valid number
 */
export function validateScore(score: bigint | number): number {
  const numScore = Number(score);

  // Validate numeric type
  if (!Number.isFinite(numScore)) {
    throw new Error(`Invalid score: not a finite number (${score})`);
  }

  // Clamp to valid range (0-100)
  if (numScore < 0 || numScore > 100) {
    eventLogger.warn(
      { score: numScore, clamped: Math.max(0, Math.min(100, numScore)) },
      "Score outside expected range (0-100), clamping to valid range"
    );
    return Math.max(0, Math.min(100, numScore));
  }

  return numScore;
}

/**
 * Validate agent ID
 *
 * Security: Ensures agent IDs are positive and within PostgreSQL bigint limits
 * to prevent database errors and display issues.
 *
 * @param agentId - Agent ID from blockchain event
 * @returns Validated agent ID
 * @throws Error if agent ID is invalid
 */
export function validateAgentId(agentId: bigint): bigint {
  // Check for negative values
  if (agentId < 0n) {
    throw new Error(`Invalid agentId: cannot be negative (${agentId})`);
  }

  // PostgreSQL bigint maximum: 2^63 - 1
  const MAX_BIGINT = 9223372036854775807n;
  if (agentId > MAX_BIGINT) {
    throw new Error(
      `Invalid agentId: exceeds PostgreSQL bigint limit (${agentId})`
    );
  }

  return agentId;
}

/**
 * Validate feedback index
 *
 * Security: Ensures feedback indexes are non-negative.
 *
 * @param feedbackIndex - Feedback index from blockchain event
 * @returns Validated feedback index
 * @throws Error if feedback index is invalid
 */
export function validateFeedbackIndex(
  feedbackIndex: bigint | number
): number {
  const numIndex = Number(feedbackIndex);

  if (!Number.isFinite(numIndex) || numIndex < 0) {
    throw new Error(
      `Invalid feedbackIndex: must be non-negative integer (${feedbackIndex})`
    );
  }

  return numIndex;
}

// ============================================================================
// URI VALIDATION
// ============================================================================

/**
 * Validate URI field (tokenURI, fileUri, responseUri, requestUri)
 *
 * Security:
 * - Prevents SSRF attacks by blocking internal network URLs
 * - Enforces maximum length to prevent storage abuse
 * - Validates URL format
 * - Allows IPFS CIDs and other decentralized storage identifiers
 *
 * @param uri - URI to validate
 * @param fieldName - Field name for error messages
 * @returns Validated URI (or undefined if input is undefined)
 * @throws Error if URI is invalid
 */
export function validateUri(
  uri: string | undefined,
  fieldName: string
): string | undefined {
  if (!uri) {
    return undefined;
  }

  // Check length (prevent storage abuse)
  const MAX_URI_LENGTH = 2048; // RFC 2616 recommended max URL length
  if (uri.length > MAX_URI_LENGTH) {
    throw new Error(
      `${fieldName} exceeds maximum length (${MAX_URI_LENGTH} chars): ${uri.length} chars`
    );
  }

  // Try parsing as URL
  try {
    const parsed = new URL(uri);

    // Whitelist safe protocols
    const ALLOWED_PROTOCOLS = ["https:", "ipfs:", "ar:", "ipns:"];
    if (!ALLOWED_PROTOCOLS.includes(parsed.protocol)) {
      eventLogger.warn(
        { uri, protocol: parsed.protocol, fieldName },
        `Suspicious URI protocol in ${fieldName}`
      );
    }

    // SSRF Protection: Block internal networks
    const BLOCKED_HOSTNAMES = [
      "localhost",
      "127.0.0.1",
      "0.0.0.0",
      "[::]",
      "[::1]",
    ];

    if (BLOCKED_HOSTNAMES.includes(parsed.hostname)) {
      throw new Error(
        `${fieldName} points to localhost (SSRF protection): ${uri}`
      );
    }

    // Block private IP ranges (RFC 1918)
    if (
      parsed.hostname.startsWith("192.168.") ||
      parsed.hostname.startsWith("10.") ||
      parsed.hostname.match(/^172\.(1[6-9]|2[0-9]|3[01])\./)
    ) {
      throw new Error(
        `${fieldName} points to internal network (SSRF protection): ${uri}`
      );
    }
  } catch (error) {
    // Not a valid URL - might be IPFS CID (QmXxx...) or other identifier
    if (uri.startsWith("Qm") || uri.startsWith("bafy")) {
      // Valid IPFS CID format (basic check)
      eventLogger.debug(
        { uri, fieldName },
        `Non-URL format detected (likely IPFS CID)`
      );
    } else if (error instanceof Error && error.message.includes("SSRF")) {
      // Re-throw SSRF errors
      throw error;
    } else {
      // Log warning but allow (could be valid identifier)
      eventLogger.debug(
        { uri, fieldName, error: (error as Error).message },
        `Could not parse ${fieldName} as URL, storing as-is`
      );
    }
  }

  return uri;
}

// ============================================================================
// HASH VALIDATION
// ============================================================================

/**
 * Validate bytes32 hash (feedbackHash, responseHash, requestHash)
 *
 * Security: Ensures hashes are valid 32-byte hex strings to prevent
 * database constraint violations and downstream processing errors.
 *
 * @param hash - Hash value from blockchain event
 * @param fieldName - Field name for error messages
 * @returns Validated hash (0x-prefixed hex string)
 * @throws Error if hash is invalid
 */
export function validateBytes32Hash(hash: Hex, fieldName: string): string {
  // Check format: must be 0x followed by 64 hex characters (32 bytes)
  if (!hash || hash.length !== 66 || !hash.startsWith("0x")) {
    throw new Error(
      `Invalid ${fieldName}: must be 32-byte hex string (0x + 64 hex chars), got length ${hash?.length}`
    );
  }

  // Validate hex characters
  if (!/^0x[0-9a-fA-F]{64}$/.test(hash)) {
    throw new Error(
      `Invalid ${fieldName}: contains non-hex characters (${hash})`
    );
  }

  return hash;
}

// ============================================================================
// STRING VALIDATION
// ============================================================================

/**
 * Validate metadata key
 *
 * Security: Enforces length limits and character restrictions to prevent
 * storage abuse and potential injection attacks.
 *
 * @param key - Metadata key from MetadataSet event
 * @returns Validated key
 * @throws Error if key is invalid
 */
export function validateMetadataKey(key: string): string {
  const MAX_KEY_LENGTH = 255;

  if (!key || key.length === 0) {
    throw new Error("Metadata key cannot be empty");
  }

  if (key.length > MAX_KEY_LENGTH) {
    throw new Error(
      `Metadata key exceeds maximum length (${MAX_KEY_LENGTH} chars): ${key.length} chars`
    );
  }

  // Check for null bytes (common SQLi technique)
  if (key.includes("\0")) {
    throw new Error("Metadata key contains null bytes");
  }

  return key;
}

/**
 * Validate and sanitize metadata value
 *
 * Security: Enforces length limits and validates UTF-8 encoding to prevent
 * storage abuse and encoding issues.
 *
 * @param value - Metadata value from MetadataSet event (bytes)
 * @returns Validated string
 * @throws Error if value is invalid
 */
export function validateMetadataValue(
  value: Uint8Array | string
): string {
  const MAX_VALUE_LENGTH = 10000; // 10KB limit

  let strValue: string;

  if (value instanceof Uint8Array) {
    // Validate UTF-8 encoding BEFORE conversion using fatal decoder
    try {
      const decoder = new TextDecoder("utf-8", { fatal: true });
      strValue = decoder.decode(value);
    } catch {
      throw new Error("Metadata value contains invalid UTF-8");
    }
  } else {
    // String input is already valid UTF-8 by JavaScript guarantee
    strValue = value;
  }

  // Check for null bytes
  if (strValue.includes("\0")) {
    throw new Error("Metadata value contains null bytes");
  }

  // Enforce length limit
  if (strValue.length > MAX_VALUE_LENGTH) {
    throw new Error(
      `Metadata value exceeds maximum length (${MAX_VALUE_LENGTH} chars): ${strValue.length} chars`
    );
  }

  return strValue;
}

/**
 * Validate tag value (tag1, tag2 from NewFeedback)
 *
 * Security: Validates bytes32 tag format and enforces length limits.
 *
 * @param tag - Tag value from blockchain event (bytes32)
 * @param fieldName - Field name for error messages
 * @returns Validated tag string (or undefined if empty)
 */
export function validateTag(
  tag: Hex | undefined,
  fieldName: string
): string | undefined {
  if (!tag) {
    return undefined;
  }

  // Empty bytes32 (0x0000...0000)
  if (tag === "0x" + "0".repeat(64)) {
    return undefined;
  }

  // Validate format
  return validateBytes32Hash(tag, fieldName);
}
