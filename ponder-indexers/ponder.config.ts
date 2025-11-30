import { createConfig } from "@ponder/core";
import { http, fallback, type Transport } from "viem";
import { loadBalance, rateLimit } from "@ponder/utils";

import IdentityRegistryAbi from "./abis/IdentityRegistry.json";
import ReputationRegistryAbi from "./abis/ReputationRegistry.json";
import ValidationRegistryAbi from "./abis/ValidationRegistry.json";
import { getEnv, getConfiguredChains, type EnvConfig } from "./src/env-validation";
import { logRpcConfig, logRpcSkipped, logConfigValidated, logConfigError, configLogger } from "./src/logger";

// ============================================================================
// ENVIRONMENT VALIDATION
// ============================================================================

let env: EnvConfig;
try {
  env = getEnv();
  const configuredChains = getConfiguredChains(env);
  logConfigValidated(configuredChains);
} catch (error) {
  logConfigError(error as Error);
  throw error;
}

// ============================================================================
// RPC CONFIGURATION
// ============================================================================

// Rate limits per provider (requests/second)
const RATE_LIMITS = {
  alchemy: env.RPC_RATE_LIMIT_ALCHEMY,
  infura: env.RPC_RATE_LIMIT_INFURA,
  quiknode: env.RPC_RATE_LIMIT_QUIKNODE,
  ankr: env.RPC_RATE_LIMIT_ANKR,
};

// Ranking configuration (health checks for smart failover)
const RANKING_CONFIG = {
  interval: env.RPC_RANK_INTERVAL,
  sampleCount: env.RPC_RANK_SAMPLE_COUNT,
  timeout: env.RPC_RANK_TIMEOUT,
};

/**
 * Get RPC URLs for a chain from validated environment
 */
function getRpcUrls(chainPrefix: string): {
  alchemy?: string;
  infura?: string;
  quiknode?: string;
  ankr?: string;
  legacy?: string;
} {
  const envKey = (suffix: string) => `${chainPrefix}_${suffix}` as keyof EnvConfig;

  return {
    alchemy: env[envKey("RPC_ALCHEMY")] as string | undefined,
    infura: env[envKey("RPC_INFURA")] as string | undefined,
    quiknode: env[envKey("RPC_QUIKNODE")] as string | undefined,
    ankr: env[envKey("RPC_ANKR")] as string | undefined,
    legacy: env[envKey("RPC_URL")] as string | undefined,
  };
}

/**
 * Creates a resilient transport with failover, load balancing, and smart ranking.
 *
 * Architecture with 4 providers:
 * - fallback() with rank: automatically promotes faster/more stable providers
 * - loadBalance() for Primary Pool (Alchemy + Infura) and Fallback Pool (QuikNode + Ankr)
 * - rateLimit() for each individual provider
 *
 * @param chainPrefix - Environment variable prefix (e.g., "ETHEREUM_SEPOLIA")
 * @returns Transport or null if no RPC is configured
 */
function createResilientTransport(chainPrefix: string): Transport | null {
  const urls = getRpcUrls(chainPrefix);

  // Create rate-limited transports for each available provider
  const transports: Transport[] = [];

  if (urls.alchemy) {
    transports.push(
      rateLimit(http(urls.alchemy), { requestsPerSecond: RATE_LIMITS.alchemy })
    );
  }
  if (urls.infura) {
    transports.push(
      rateLimit(http(urls.infura), { requestsPerSecond: RATE_LIMITS.infura })
    );
  }
  if (urls.quiknode) {
    transports.push(
      rateLimit(http(urls.quiknode), { requestsPerSecond: RATE_LIMITS.quiknode })
    );
  }
  if (urls.ankr) {
    transports.push(
      rateLimit(http(urls.ankr), { requestsPerSecond: RATE_LIMITS.ankr })
    );
  }

  // Fallback to legacy URL if no multi-provider URLs configured
  if (transports.length === 0 && urls.legacy) {
    logRpcConfig(chainPrefix, "legacy single provider");
    return http(urls.legacy);
  }

  if (transports.length === 0) {
    logRpcSkipped(chainPrefix);
    return null;
  }

  // 1 provider: use directly (no fallback needed)
  if (transports.length === 1) {
    logRpcConfig(chainPrefix, "single provider");
    return transports[0]!;
  }

  // 2-3 providers: fallback with smart ranking
  if (transports.length <= 3) {
    logRpcConfig(chainPrefix, "fallback with ranking", transports.length);
    return fallback(transports, {
      retryCount: 3,
      retryDelay: 1000,
      rank: {
        interval: RANKING_CONFIG.interval,
        sampleCount: RANKING_CONFIG.sampleCount,
        timeout: RANKING_CONFIG.timeout,
        weights: {
          latency: 0.3, // 30% weight on latency
          stability: 0.7, // 70% weight on stability
        },
      },
    });
  }

  // 4+ providers: load balance between Primary Pool and Fallback Pool with smart ranking
  const mid = Math.ceil(transports.length / 2);
  const primaryPool = loadBalance(transports.slice(0, mid));
  const fallbackPool = loadBalance(transports.slice(mid));

  logRpcConfig(chainPrefix, "load-balanced failover with ranking", transports.length);

  return fallback([primaryPool, fallbackPool], {
    retryCount: 3,
    retryDelay: 1000,
    rank: {
      interval: RANKING_CONFIG.interval,
      sampleCount: RANKING_CONFIG.sampleCount,
      timeout: RANKING_CONFIG.timeout,
      weights: {
        latency: 0.3,
        stability: 0.7,
      },
    },
  });
}

// ============================================================================
// NETWORK CONFIGURATION
// ============================================================================

interface NetworkConfig {
  chainId: number;
  name: string;
}

const networks: Record<string, NetworkConfig> = {
  // Testnets
  ethereumSepolia: {
    chainId: 11155111,
    name: "Ethereum Sepolia",
  },
  baseSepolia: {
    chainId: 84532,
    name: "Base Sepolia",
  },
  lineaSepolia: {
    chainId: 59141,
    name: "Linea Sepolia",
  },
  polygonAmoy: {
    chainId: 80002,
    name: "Polygon Amoy",
  },
  // Mainnets
  ethereumMainnet: {
    chainId: 1,
    name: "Ethereum Mainnet",
  },
  baseMainnet: {
    chainId: 8453,
    name: "Base Mainnet",
  },
  lineaMainnet: {
    chainId: 59144,
    name: "Linea Mainnet",
  },
};

// ============================================================================
// CONTRACT ADDRESSES (from validated environment)
// ============================================================================

/**
 * Get contract address from validated env with default fallback
 */
function getContractAddress(key: keyof EnvConfig): `0x${string}` {
  const value = env[key] as string | undefined;
  return (value || "0x0000000000000000000000000000000000000000") as `0x${string}`;
}

const contracts: Record<string, Record<string, `0x${string}`>> = {
  // Testnets
  ethereumSepolia: {
    identity: getContractAddress("ETHEREUM_SEPOLIA_IDENTITY_ADDRESS"),
    reputation: getContractAddress("ETHEREUM_SEPOLIA_REPUTATION_ADDRESS"),
    validation: getContractAddress("ETHEREUM_SEPOLIA_VALIDATION_ADDRESS"),
  },
  baseSepolia: {
    identity: getContractAddress("BASE_SEPOLIA_IDENTITY_ADDRESS"),
    reputation: getContractAddress("BASE_SEPOLIA_REPUTATION_ADDRESS"),
    validation: getContractAddress("BASE_SEPOLIA_VALIDATION_ADDRESS"),
  },
  lineaSepolia: {
    identity: getContractAddress("LINEA_SEPOLIA_IDENTITY_ADDRESS"),
    reputation: getContractAddress("LINEA_SEPOLIA_REPUTATION_ADDRESS"),
    validation: getContractAddress("LINEA_SEPOLIA_VALIDATION_ADDRESS"),
  },
  polygonAmoy: {
    identity: getContractAddress("POLYGON_AMOY_IDENTITY_ADDRESS"),
    reputation: getContractAddress("POLYGON_AMOY_REPUTATION_ADDRESS"),
    validation: getContractAddress("POLYGON_AMOY_VALIDATION_ADDRESS"),
  },
  // Mainnets
  ethereumMainnet: {
    identity: getContractAddress("ETHEREUM_MAINNET_IDENTITY_ADDRESS"),
    reputation: getContractAddress("ETHEREUM_MAINNET_REPUTATION_ADDRESS"),
    validation: getContractAddress("ETHEREUM_MAINNET_VALIDATION_ADDRESS"),
  },
  baseMainnet: {
    identity: getContractAddress("BASE_MAINNET_IDENTITY_ADDRESS"),
    reputation: getContractAddress("BASE_MAINNET_REPUTATION_ADDRESS"),
    validation: getContractAddress("BASE_MAINNET_VALIDATION_ADDRESS"),
  },
  lineaMainnet: {
    identity: getContractAddress("LINEA_MAINNET_IDENTITY_ADDRESS"),
    reputation: getContractAddress("LINEA_MAINNET_REPUTATION_ADDRESS"),
    validation: getContractAddress("LINEA_MAINNET_VALIDATION_ADDRESS"),
  },
};

// ============================================================================
// START BLOCKS (from validated environment)
// ============================================================================

const startBlocks: Record<string, number> = {
  // Testnets
  ethereumSepolia: env.ETHEREUM_SEPOLIA_START_BLOCK ?? 0,
  baseSepolia: env.BASE_SEPOLIA_START_BLOCK ?? 0,
  lineaSepolia: env.LINEA_SEPOLIA_START_BLOCK ?? 0,
  polygonAmoy: env.POLYGON_AMOY_START_BLOCK ?? 0,
  // Mainnets
  ethereumMainnet: env.ETHEREUM_MAINNET_START_BLOCK ?? 0,
  baseMainnet: env.BASE_MAINNET_START_BLOCK ?? 0,
  lineaMainnet: env.LINEA_MAINNET_START_BLOCK ?? 0,
};

// ============================================================================
// DYNAMIC NETWORK AND CONTRACT CONFIGURATION
// ============================================================================

// Map of network key to env prefix
const networkEnvPrefixes: Record<string, string> = {
  // Testnets
  ethereumSepolia: "ETHEREUM_SEPOLIA",
  baseSepolia: "BASE_SEPOLIA",
  lineaSepolia: "LINEA_SEPOLIA",
  polygonAmoy: "POLYGON_AMOY",
  // Mainnets
  ethereumMainnet: "ETHEREUM_MAINNET",
  baseMainnet: "BASE_MAINNET",
  lineaMainnet: "LINEA_MAINNET",
};

// Build networks dynamically (only include those with RPC configured)
const configuredNetworks: Record<
  string,
  { chainId: number; transport: Transport }
> = {};
const enabledNetworkKeys: string[] = [];

for (const [networkKey, envPrefix] of Object.entries(networkEnvPrefixes)) {
  const transport = createResilientTransport(envPrefix);
  const networkConfig = networks[networkKey];
  if (transport && networkConfig) {
    configuredNetworks[networkKey] = {
      chainId: networkConfig.chainId,
      transport,
    };
    enabledNetworkKeys.push(networkKey);
  }
}

// Log final configuration summary
configLogger.info({
  enabledNetworks: enabledNetworkKeys,
  totalNetworks: enabledNetworkKeys.length,
}, "Ponder configuration complete");

// Build contracts dynamically (only for enabled networks)
type ContractConfig = {
  network: string;
  abi: unknown;
  address: `0x${string}`;
  startBlock: number;
};

const configuredContracts: Record<string, ContractConfig> = {};

for (const networkKey of enabledNetworkKeys) {
  const contractAddrs = contracts[networkKey];
  const startBlock = startBlocks[networkKey] ?? 0;
  const pascalNetworkKey =
    networkKey.charAt(0).toUpperCase() + networkKey.slice(1);

  // Skip if no contract addresses for this network
  if (!contractAddrs) continue;

  // Identity Registry
  configuredContracts[`IdentityRegistry${pascalNetworkKey}`] = {
    network: networkKey,
    abi: IdentityRegistryAbi,
    address: contractAddrs["identity"]!,
    startBlock,
  };

  // Reputation Registry
  configuredContracts[`ReputationRegistry${pascalNetworkKey}`] = {
    network: networkKey,
    abi: ReputationRegistryAbi,
    address: contractAddrs["reputation"]!,
    startBlock,
  };

  // Validation Registry
  configuredContracts[`ValidationRegistry${pascalNetworkKey}`] = {
    network: networkKey,
    abi: ValidationRegistryAbi,
    address: contractAddrs["validation"]!,
    startBlock,
  };
}

// ============================================================================
// PONDER CONFIGURATION
// ============================================================================

export default createConfig({
  // Database configuration - uses PostgreSQL from validated env
  database: {
    kind: "postgres",
    connectionString: env.DATABASE_URL,
  },

  // Network configurations (dynamically built)
  networks: configuredNetworks,

  // Contract configurations (dynamically built)
  contracts: configuredContracts,
});
