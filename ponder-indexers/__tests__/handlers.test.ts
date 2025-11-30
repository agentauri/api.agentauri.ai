/**
 * Handler Helper Functions Tests
 *
 * Tests for the helper functions used in event handlers
 */
import { describe, it, expect } from "vitest";
import { generateEventId, bytes32ToHex, CHAIN_IDS, REGISTRIES } from "../helpers";

describe("CHAIN_IDS constants", () => {
  it("should have correct Ethereum Sepolia chain ID", () => {
    expect(CHAIN_IDS.ETHEREUM_SEPOLIA).toBe(11155111n);
  });

  it("should have correct Base Sepolia chain ID", () => {
    expect(CHAIN_IDS.BASE_SEPOLIA).toBe(84532n);
  });

  it("should have correct Linea Sepolia chain ID", () => {
    expect(CHAIN_IDS.LINEA_SEPOLIA).toBe(59141n);
  });

  it("should have correct Polygon Amoy chain ID", () => {
    expect(CHAIN_IDS.POLYGON_AMOY).toBe(80002n);
  });
});

describe("REGISTRIES constants", () => {
  it("should have identity registry", () => {
    expect(REGISTRIES.IDENTITY).toBe("identity");
  });

  it("should have reputation registry", () => {
    expect(REGISTRIES.REPUTATION).toBe("reputation");
  });

  it("should have validation registry", () => {
    expect(REGISTRIES.VALIDATION).toBe("validation");
  });
});

describe("generateEventId", () => {
  it("should generate correct event ID format", () => {
    const eventId = generateEventId(
      REGISTRIES.IDENTITY,
      CHAIN_IDS.ETHEREUM_SEPOLIA,
      "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" as `0x${string}`,
      5
    );

    expect(eventId).toBe(
      "identity-11155111-0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef-5"
    );
  });

  it("should include registry in event ID to prevent collisions", () => {
    const txHash = "0xabc123" as `0x${string}`;
    const chainId = CHAIN_IDS.BASE_SEPOLIA;
    const logIndex = 0;

    const identityEventId = generateEventId(REGISTRIES.IDENTITY, chainId, txHash, logIndex);
    const reputationEventId = generateEventId(REGISTRIES.REPUTATION, chainId, txHash, logIndex);
    const validationEventId = generateEventId(REGISTRIES.VALIDATION, chainId, txHash, logIndex);

    // All three should be different due to registry prefix
    expect(identityEventId).not.toBe(reputationEventId);
    expect(identityEventId).not.toBe(validationEventId);
    expect(reputationEventId).not.toBe(validationEventId);
  });

  it("should produce unique IDs for different log indices", () => {
    const txHash = "0xabc123" as `0x${string}`;
    const chainId = CHAIN_IDS.ETHEREUM_SEPOLIA;

    const eventId0 = generateEventId(REGISTRIES.IDENTITY, chainId, txHash, 0);
    const eventId1 = generateEventId(REGISTRIES.IDENTITY, chainId, txHash, 1);
    const eventId2 = generateEventId(REGISTRIES.IDENTITY, chainId, txHash, 2);

    expect(eventId0).not.toBe(eventId1);
    expect(eventId1).not.toBe(eventId2);
    expect(eventId0).not.toBe(eventId2);
  });

  it("should produce unique IDs for different chains", () => {
    const txHash = "0xabc123" as `0x${string}`;
    const logIndex = 0;

    const ethereumEventId = generateEventId(REGISTRIES.IDENTITY, CHAIN_IDS.ETHEREUM_SEPOLIA, txHash, logIndex);
    const baseEventId = generateEventId(REGISTRIES.IDENTITY, CHAIN_IDS.BASE_SEPOLIA, txHash, logIndex);
    const lineaEventId = generateEventId(REGISTRIES.IDENTITY, CHAIN_IDS.LINEA_SEPOLIA, txHash, logIndex);

    expect(ethereumEventId).not.toBe(baseEventId);
    expect(baseEventId).not.toBe(lineaEventId);
    expect(ethereumEventId).not.toBe(lineaEventId);
  });

  it("should handle bigint chain IDs correctly", () => {
    const eventId = generateEventId(
      REGISTRIES.REPUTATION,
      BigInt("999999999999"),
      "0xdef456" as `0x${string}`,
      10
    );

    expect(eventId).toContain("999999999999");
    expect(typeof eventId).toBe("string");
  });
});

describe("bytes32ToHex", () => {
  it("should return hex string as-is", () => {
    const input = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" as `0x${string}`;
    const result = bytes32ToHex(input);
    expect(result).toBe(input);
  });

  it("should preserve the 0x prefix", () => {
    const input = "0xdeadbeef" as `0x${string}`;
    const result = bytes32ToHex(input);
    expect(result).toMatch(/^0x/);
  });

  it("should work with full bytes32 length", () => {
    const fullBytes32 = "0x0000000000000000000000000000000000000000000000000000000000000000" as `0x${string}`;
    const result = bytes32ToHex(fullBytes32);
    expect(result).toBe(fullBytes32);
    expect(result.length).toBe(66); // 0x + 64 hex chars
  });
});
