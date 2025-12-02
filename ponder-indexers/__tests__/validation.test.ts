/**
 * Unit Tests for Input Validation Utilities
 *
 * Tests all validation functions to ensure they correctly validate
 * blockchain event parameters and prevent invalid data from entering
 * the Event Store.
 */

import { describe, it, expect } from "vitest";
import {
  validateAndNormalizeAddress,
  validateScore,
  validateAgentId,
  validateFeedbackIndex,
  validateUri,
  validateBytes32Hash,
  validateMetadataKey,
  validateMetadataValue,
  validateTag,
} from "../src/validation";

// ============================================================================
// ADDRESS VALIDATION TESTS
// ============================================================================

describe("validateAndNormalizeAddress", () => {
  it("should accept valid addresses", () => {
    const validAddresses = [
      "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb5",
      "0x0000000000000000000000000000000000000000", // Zero address
      "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", // Max address
    ];

    validAddresses.forEach((addr) => {
      expect(validateAndNormalizeAddress(addr)).toBe(addr.toLowerCase());
    });
  });

  it("should reject malformed addresses", () => {
    const invalidAddresses = [
      "0xINVALID",
      "0x1234", // Too short
      "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb5XXX", // Too long
      "742d35Cc6634C0532925a3b844Bc9e7595f0bEb5", // Missing 0x
      "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEg5", // Invalid hex (g)
    ];

    invalidAddresses.forEach((addr) => {
      expect(() => validateAndNormalizeAddress(addr)).toThrow();
    });
  });

  it("should reject non-string types", () => {
    expect(() => validateAndNormalizeAddress(null)).toThrow("Invalid address type");
    expect(() => validateAndNormalizeAddress(undefined)).toThrow("Invalid address type");
    expect(() => validateAndNormalizeAddress(123)).toThrow("Invalid address type");
    expect(() => validateAndNormalizeAddress({})).toThrow("Invalid address type");
  });

  it("should include field name in error messages", () => {
    try {
      validateAndNormalizeAddress("invalid", "clientAddress");
    } catch (error) {
      expect((error as Error).message).toContain("clientAddress");
    }
  });
});

// ============================================================================
// NUMERIC VALIDATION TESTS
// ============================================================================

describe("validateScore", () => {
  it("should accept valid scores (0-100)", () => {
    expect(validateScore(0)).toBe(0);
    expect(validateScore(50)).toBe(50);
    expect(validateScore(100)).toBe(100);
    expect(validateScore(0n)).toBe(0); // bigint
    expect(validateScore(50n)).toBe(50);
    expect(validateScore(100n)).toBe(100);
  });

  it("should clamp scores below 0", () => {
    expect(validateScore(-1)).toBe(0);
    expect(validateScore(-100)).toBe(0);
    expect(validateScore(-1n)).toBe(0);
  });

  it("should clamp scores above 100", () => {
    expect(validateScore(101)).toBe(100);
    expect(validateScore(255)).toBe(100);
    expect(validateScore(1000)).toBe(100);
    expect(validateScore(255n)).toBe(100);
  });

  it("should reject non-finite numbers", () => {
    expect(() => validateScore(NaN)).toThrow("not a finite number");
    expect(() => validateScore(Infinity)).toThrow("not a finite number");
    expect(() => validateScore(-Infinity)).toThrow("not a finite number");
  });
});

describe("validateAgentId", () => {
  it("should accept valid agent IDs", () => {
    expect(validateAgentId(0n)).toBe(0n);
    expect(validateAgentId(1n)).toBe(1n);
    expect(validateAgentId(123456789n)).toBe(123456789n);
  });

  it("should reject negative agent IDs", () => {
    expect(() => validateAgentId(-1n)).toThrow("cannot be negative");
    expect(() => validateAgentId(-123n)).toThrow("cannot be negative");
  });

  it("should reject agent IDs exceeding PostgreSQL bigint limit", () => {
    const MAX_BIGINT = 9223372036854775807n;
    expect(validateAgentId(MAX_BIGINT)).toBe(MAX_BIGINT); // Max valid
    expect(() => validateAgentId(MAX_BIGINT + 1n)).toThrow("exceeds PostgreSQL bigint limit");
  });
});

describe("validateFeedbackIndex", () => {
  it("should accept valid feedback indexes", () => {
    expect(validateFeedbackIndex(0)).toBe(0);
    expect(validateFeedbackIndex(1)).toBe(1);
    expect(validateFeedbackIndex(999n)).toBe(999);
  });

  it("should reject negative feedback indexes", () => {
    expect(() => validateFeedbackIndex(-1)).toThrow("must be non-negative");
    expect(() => validateFeedbackIndex(-1n)).toThrow("must be non-negative");
  });

  it("should reject non-finite numbers", () => {
    expect(() => validateFeedbackIndex(NaN)).toThrow("must be non-negative");
    expect(() => validateFeedbackIndex(Infinity)).toThrow("must be non-negative");
  });
});

// ============================================================================
// URI VALIDATION TESTS
// ============================================================================

describe("validateUri", () => {
  it("should accept valid HTTPS URLs", () => {
    const validUrls = [
      "https://example.com/file.json",
      "https://ipfs.io/ipfs/QmXxxxxx",
      "https://arweave.net/xxx",
    ];

    validUrls.forEach((url) => {
      expect(validateUri(url, "fileUri")).toBe(url);
    });
  });

  it("should accept IPFS CIDs", () => {
    const ipfsCids = [
      "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco",
      "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    ];

    ipfsCids.forEach((cid) => {
      expect(validateUri(cid, "fileUri")).toBe(cid);
    });
  });

  it("should accept IPFS and Arweave URLs", () => {
    expect(validateUri("ipfs://QmXxxxxx", "fileUri")).toBe("ipfs://QmXxxxxx");
    expect(validateUri("ar://xxxxx", "fileUri")).toBe("ar://xxxxx");
  });

  it("should return undefined for empty/undefined input", () => {
    expect(validateUri(undefined, "fileUri")).toBeUndefined();
    expect(validateUri("", "fileUri")).toBeUndefined();
  });

  it("should reject URIs exceeding maximum length", () => {
    const longUri = "https://example.com/" + "x".repeat(2048);
    expect(() => validateUri(longUri, "fileUri")).toThrow("exceeds maximum length");
  });

  it("should reject localhost URLs (SSRF protection)", () => {
    const localhostUrls = [
      "http://localhost/file.json",
      "https://localhost:8080/api",
      "http://127.0.0.1/internal",
      "http://0.0.0.0:3000/admin",
    ];

    localhostUrls.forEach((url) => {
      expect(() => validateUri(url, "fileUri")).toThrow("SSRF protection");
    });
  });

  it("should reject private IP ranges (SSRF protection)", () => {
    const privateIps = [
      "http://192.168.1.1/internal",
      "http://10.0.0.1/admin",
      "http://172.16.0.1/api",
      "http://172.31.255.255/data",
    ];

    privateIps.forEach((url) => {
      expect(() => validateUri(url, "fileUri")).toThrow("SSRF protection");
    });
  });

  it("should allow public IPs", () => {
    const publicIps = [
      "http://8.8.8.8/dns",
      "http://1.1.1.1/cloudflare",
    ];

    publicIps.forEach((url) => {
      expect(validateUri(url, "fileUri")).toBe(url);
    });
  });
});

// ============================================================================
// HASH VALIDATION TESTS
// ============================================================================

describe("validateBytes32Hash", () => {
  it("should accept valid bytes32 hashes", () => {
    const validHashes = [
      "0x" + "0".repeat(64), // Zero hash
      "0x" + "f".repeat(64), // Max hash
      "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    ];

    validHashes.forEach((hash) => {
      expect(validateBytes32Hash(hash as `0x${string}`, "fileHash")).toBe(hash);
    });
  });

  it("should reject hashes with incorrect length", () => {
    const invalidHashes = [
      "0x1234", // Too short
      "0x" + "1".repeat(65), // Too long
      "0x" + "1".repeat(63), // Too short by 1
    ];

    invalidHashes.forEach((hash) => {
      expect(() => validateBytes32Hash(hash as `0x${string}`, "fileHash")).toThrow(
        "must be 32-byte hex string"
      );
    });
  });

  it("should reject hashes without 0x prefix", () => {
    const hash = "1".repeat(64);
    expect(() => validateBytes32Hash(hash as `0x${string}`, "fileHash")).toThrow(
      "must be 32-byte hex string"
    );
  });

  it("should reject hashes with non-hex characters", () => {
    const invalidHashes = [
      "0x" + "g".repeat(64), // Invalid hex (g)
      "0x" + "z".repeat(64), // Invalid hex (z)
      "0x" + " ".repeat(64), // Spaces
    ];

    invalidHashes.forEach((hash) => {
      expect(() => validateBytes32Hash(hash as `0x${string}`, "fileHash")).toThrow(
        "contains non-hex characters"
      );
    });
  });

  it("should include field name in error messages", () => {
    try {
      validateBytes32Hash("0xINVALID" as `0x${string}`, "responseHash");
    } catch (error) {
      expect((error as Error).message).toContain("responseHash");
    }
  });
});

// ============================================================================
// STRING VALIDATION TESTS
// ============================================================================

describe("validateMetadataKey", () => {
  it("should accept valid metadata keys", () => {
    const validKeys = [
      "name",
      "description",
      "capabilities.mcp.endpoint",
      "key-with-dashes",
      "key_with_underscores",
    ];

    validKeys.forEach((key) => {
      expect(validateMetadataKey(key)).toBe(key);
    });
  });

  it("should reject empty keys", () => {
    expect(() => validateMetadataKey("")).toThrow("cannot be empty");
  });

  it("should reject keys exceeding maximum length", () => {
    const longKey = "x".repeat(256);
    expect(() => validateMetadataKey(longKey)).toThrow("exceeds maximum length");
  });

  it("should reject keys with null bytes", () => {
    expect(() => validateMetadataKey("key\0withNullByte")).toThrow("contains null bytes");
  });
});

describe("validateMetadataValue", () => {
  it("should accept valid string values", () => {
    const validValues = [
      "simple value",
      "Value with spaces and punctuation!",
      '{"json": "value"}',
      "Multi\nline\nvalue",
    ];

    validValues.forEach((value) => {
      expect(validateMetadataValue(value)).toBe(value);
    });
  });

  it("should accept valid byte values (convert to UTF-8)", () => {
    const bytes = new TextEncoder().encode("Hello, World!");
    expect(validateMetadataValue(bytes)).toBe("Hello, World!");
  });

  it("should reject values exceeding maximum length", () => {
    const longValue = "x".repeat(10001);
    expect(() => validateMetadataValue(longValue)).toThrow("exceeds maximum length");
  });

  it("should reject values with null bytes", () => {
    expect(() => validateMetadataValue("value\0withNullByte")).toThrow(
      "contains null bytes"
    );
  });

  it("should reject invalid UTF-8", () => {
    // Create invalid UTF-8 sequence
    const invalidUtf8 = new Uint8Array([0xff, 0xfe, 0xfd]);

    // Pass Uint8Array directly to validate UTF-8 encoding properly
    expect(() => validateMetadataValue(invalidUtf8)).toThrow("invalid UTF-8");
  });
});

describe("validateTag", () => {
  it("should accept valid tags", () => {
    const validTag = "0x" + "1".repeat(64);
    expect(validateTag(validTag as `0x${string}`, "tag1")).toBe(validTag);
  });

  it("should return undefined for empty tags", () => {
    const emptyTag = "0x" + "0".repeat(64);
    expect(validateTag(emptyTag as `0x${string}`, "tag1")).toBeUndefined();
  });

  it("should return undefined for undefined input", () => {
    expect(validateTag(undefined, "tag1")).toBeUndefined();
  });

  it("should validate tag format", () => {
    const invalidTag = "0xINVALID";
    expect(() => validateTag(invalidTag as `0x${string}`, "tag1")).toThrow(
      "must be 32-byte hex string"
    );
  });
});

// ============================================================================
// INTEGRATION TESTS (Multiple Validations)
// ============================================================================

describe("Integration: Validate NewFeedback Event", () => {
  it("should validate all fields from a valid NewFeedback event", () => {
    const mockEvent = {
      agentId: 123n,
      clientAddress: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb5",
      score: 85n,
      tag1: "0x" + "1".repeat(64),
      tag2: "0x" + "0".repeat(64), // Empty tag
      feedbackUri: "https://ipfs.io/ipfs/QmXxxxxx",
      feedbackHash: "0x" + "a".repeat(64),
    };

    // Validate all fields
    expect(validateAgentId(mockEvent.agentId)).toBe(123n);
    expect(validateAndNormalizeAddress(mockEvent.clientAddress, "clientAddress")).toBe(
      mockEvent.clientAddress.toLowerCase()
    );
    expect(validateScore(mockEvent.score)).toBe(85);
    expect(validateTag(mockEvent.tag1 as `0x${string}`, "tag1")).toBe(mockEvent.tag1);
    expect(validateTag(mockEvent.tag2 as `0x${string}`, "tag2")).toBeUndefined();
    expect(validateUri(mockEvent.feedbackUri, "feedbackUri")).toBe(mockEvent.feedbackUri);
    expect(validateBytes32Hash(mockEvent.feedbackHash as `0x${string}`, "feedbackHash")).toBe(
      mockEvent.feedbackHash
    );
  });

  it("should reject invalid NewFeedback event", () => {
    const mockEvent = {
      agentId: -1n, // Invalid (negative)
      clientAddress: "0xINVALID", // Invalid address
      score: 150n, // Out of range (will be clamped)
      tag1: "0xINVALID", // Invalid hash
      feedbackUri: "http://localhost/evil", // SSRF attempt
      feedbackHash: "0xINVALID", // Invalid hash
    };

    expect(() => validateAgentId(mockEvent.agentId)).toThrow();
    expect(() => validateAndNormalizeAddress(mockEvent.clientAddress, "clientAddress")).toThrow();
    expect(validateScore(mockEvent.score)).toBe(100); // Clamped to max
    expect(() => validateTag(mockEvent.tag1 as `0x${string}`, "tag1")).toThrow();
    expect(() => validateUri(mockEvent.feedbackUri, "feedbackUri")).toThrow();
    expect(() => validateBytes32Hash(mockEvent.feedbackHash as `0x${string}`, "feedbackHash")).toThrow();
  });
});
