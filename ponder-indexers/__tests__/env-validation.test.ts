/**
 * Environment Validation Tests
 *
 * Tests for the Zod-based environment variable validation
 */
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { envSchema, resetEnvCache, getConfiguredChains, type EnvConfig } from "../env-validation";

describe("envSchema", () => {
  const originalEnv = process.env;

  beforeEach(() => {
    vi.resetModules();
    process.env = { ...originalEnv };
    resetEnvCache();
  });

  afterEach(() => {
    process.env = originalEnv;
    resetEnvCache();
  });

  describe("DATABASE_URL validation", () => {
    it("should accept valid PostgreSQL URL", () => {
      const env = {
        DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://eth-sepolia.g.alchemy.com/v2/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
    });

    it("should accept postgres:// prefix", () => {
      const env = {
        DATABASE_URL: "postgres://user:pass@localhost:5432/db",
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://eth-sepolia.g.alchemy.com/v2/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
    });

    it("should reject non-PostgreSQL URL", () => {
      const env = {
        DATABASE_URL: "mysql://user:pass@localhost:3306/db",
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://eth-sepolia.g.alchemy.com/v2/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });

    it("should reject missing DATABASE_URL", () => {
      const env = {
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://eth-sepolia.g.alchemy.com/v2/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });
  });

  describe("RPC URL validation", () => {
    const validEnv = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
    };

    it("should accept HTTPS URL for Alchemy", () => {
      const env = {
        ...validEnv,
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://eth-sepolia.g.alchemy.com/v2/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
    });

    it("should reject HTTP URL (non-HTTPS)", () => {
      const env = {
        ...validEnv,
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "http://eth-sepolia.g.alchemy.com/v2/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });

    it("should accept multiple RPC providers", () => {
      const env = {
        ...validEnv,
        ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
        ETHEREUM_SEPOLIA_RPC_INFURA: "https://infura.io/key",
        BASE_SEPOLIA_RPC_QUIKNODE: "https://quiknode.pro/key",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
    });
  });

  describe("Contract address validation", () => {
    const validEnv = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
    };

    it("should accept valid Ethereum address", () => {
      const env = {
        ...validEnv,
        ETHEREUM_SEPOLIA_IDENTITY_ADDRESS: "0x1234567890123456789012345678901234567890",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
    });

    it("should reject invalid Ethereum address (too short)", () => {
      const env = {
        ...validEnv,
        ETHEREUM_SEPOLIA_IDENTITY_ADDRESS: "0x123456",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });

    it("should reject address without 0x prefix", () => {
      const env = {
        ...validEnv,
        ETHEREUM_SEPOLIA_IDENTITY_ADDRESS: "1234567890123456789012345678901234567890",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });
  });

  describe("Rate limit validation", () => {
    const validEnv = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
    };

    it("should use default rate limits when not specified", () => {
      const result = envSchema.safeParse(validEnv);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.RPC_RATE_LIMIT_ALCHEMY).toBe(25);
        expect(result.data.RPC_RATE_LIMIT_INFURA).toBe(20);
        expect(result.data.RPC_RATE_LIMIT_QUIKNODE).toBe(25);
        expect(result.data.RPC_RATE_LIMIT_ANKR).toBe(30);
      }
    });

    it("should accept custom rate limits", () => {
      const env = {
        ...validEnv,
        RPC_RATE_LIMIT_ALCHEMY: "50",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.RPC_RATE_LIMIT_ALCHEMY).toBe(50);
      }
    });

    it("should reject rate limit > 100", () => {
      const env = {
        ...validEnv,
        RPC_RATE_LIMIT_ALCHEMY: "150",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });

    it("should reject rate limit < 1", () => {
      const env = {
        ...validEnv,
        RPC_RATE_LIMIT_ALCHEMY: "0",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });
  });

  describe("Ranking configuration validation", () => {
    const validEnv = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
    };

    it("should use default ranking config when not specified", () => {
      const result = envSchema.safeParse(validEnv);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.RPC_RANK_INTERVAL).toBe(10000);
        expect(result.data.RPC_RANK_SAMPLE_COUNT).toBe(10);
        expect(result.data.RPC_RANK_TIMEOUT).toBe(2000);
      }
    });

    it("should accept custom ranking config", () => {
      const env = {
        ...validEnv,
        RPC_RANK_INTERVAL: "5000",
        RPC_RANK_SAMPLE_COUNT: "5",
        RPC_RANK_TIMEOUT: "1000",
      };

      const result = envSchema.safeParse(env);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.RPC_RANK_INTERVAL).toBe(5000);
        expect(result.data.RPC_RANK_SAMPLE_COUNT).toBe(5);
        expect(result.data.RPC_RANK_TIMEOUT).toBe(1000);
      }
    });
  });

  describe("Ponder configuration validation", () => {
    const validEnv = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
    };

    it("should default to info log level", () => {
      const result = envSchema.safeParse(validEnv);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.PONDER_LOG_LEVEL).toBe("info");
      }
    });

    it("should accept valid log levels", () => {
      const levels = ["debug", "info", "warn", "error"] as const;

      for (const level of levels) {
        const env = { ...validEnv, PONDER_LOG_LEVEL: level };
        const result = envSchema.safeParse(env);
        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data.PONDER_LOG_LEVEL).toBe(level);
        }
      }
    });

    it("should reject invalid log level", () => {
      const env = { ...validEnv, PONDER_LOG_LEVEL: "verbose" };
      const result = envSchema.safeParse(env);
      expect(result.success).toBe(false);
    });
  });
});

describe("getConfiguredChains", () => {
  it("should return empty array when no chains configured", () => {
    const env = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
    } as EnvConfig;

    const chains = getConfiguredChains(env);
    expect(chains).toEqual([]);
  });

  it("should return ethereumSepolia when Alchemy is configured", () => {
    const env = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
    } as EnvConfig;

    const chains = getConfiguredChains(env);
    expect(chains).toContain("ethereumSepolia");
  });

  it("should return multiple chains when configured", () => {
    const env = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_ALCHEMY: "https://alchemy.com/key",
      BASE_SEPOLIA_RPC_INFURA: "https://infura.io/key",
      LINEA_SEPOLIA_RPC_ANKR: "https://ankr.com/key",
    } as EnvConfig;

    const chains = getConfiguredChains(env);
    expect(chains).toContain("ethereumSepolia");
    expect(chains).toContain("baseSepolia");
    expect(chains).toContain("lineaSepolia");
    expect(chains).toHaveLength(3);
  });

  it("should detect legacy RPC_URL format", () => {
    const env = {
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db",
      ETHEREUM_SEPOLIA_RPC_URL: "https://legacy.com/key",
    } as EnvConfig;

    const chains = getConfiguredChains(env);
    expect(chains).toContain("ethereumSepolia");
  });
});
