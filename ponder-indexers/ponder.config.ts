import { createConfig, mergeAbis } from "@ponder/core";
import { http, fallback, type Transport, type Abi } from "viem";
import { loadBalance, rateLimit } from "@ponder/utils";

// Import ABIs with type assertion for strict viem types
import _ERC1967ProxyAbi from "./abis/ERC1967Proxy.json";
import _IdentityRegistryAbi from "./abis/IdentityRegistry.json";
import _ReputationRegistryAbi from "./abis/ReputationRegistry.json";
import _ValidationRegistryAbi from "./abis/ValidationRegistry.json";

const ERC1967ProxyAbi = _ERC1967ProxyAbi as unknown as Abi;
const IdentityRegistryAbi = _IdentityRegistryAbi as unknown as Abi;
const ReputationRegistryAbi = _ReputationRegistryAbi as unknown as Abi;
const ValidationRegistryAbi = _ValidationRegistryAbi as unknown as Abi;
import { getEnv, getConfiguredChains, type EnvConfig } from "./src/env-validation";
import {
  createDatabaseHealthMonitor,
  stopDatabaseHealthMonitor,
  createDeadLetterQueue,
  shutdownDeadLetterQueue,
} from "./src/database";
import {
  logRpcConfig,
  logRpcSkipped,
  logConfigValidated,
  logConfigError,
  configLogger,
  logHealthCheckStart,
  logHealthCheckResult,
  logHealthCheckSummary,
} from "./src/logger";
import { healthCheckProviders } from "./src/health-check";
import {
  CircuitBreakerManager,
  type CircuitBreakerConfig,
} from "./src/circuit-breaker";
import {
  createRuntimeHealthMonitor,
  type RuntimeHealthMonitor,
} from "./src/runtime-health-monitor";
import {
  getQuotaManager,
} from "./src/quota-tracker";
import {
  getReputationStore,
  type ReputationStore,
} from "./src/reputation-store";

// ============================================================================
// GLOBAL ERROR HANDLERS - Prevent crashes from unhandled rejections
// ============================================================================
// These handlers catch errors that would otherwise crash the entire process.
// See: https://github.com/ponder-sh/ponder/issues/861

process.on('unhandledRejection', (reason, _promise) => {
  configLogger.error({
    type: 'unhandledRejection',
    reason: reason instanceof Error ? reason.message : String(reason),
    stack: reason instanceof Error ? reason.stack : undefined,
  }, 'Unhandled promise rejection caught - preventing crash');
  // Do NOT rethrow - this prevents the crash
});

process.on('uncaughtException', (error) => {
  configLogger.error({
    type: 'uncaughtException',
    message: error.message,
    stack: error.stack,
  }, 'Uncaught exception caught - logging before potential crash');
  // Note: For uncaught exceptions, Node.js will still exit after this handler
  // unless you explicitly prevent it. But at least we log the error.
});

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
  publicnode: env.RPC_RATE_LIMIT_PUBLICNODE,
  llamanodes: env.RPC_RATE_LIMIT_LLAMANODES,
};

// Ranking configuration (health checks for smart failover)
const RANKING_CONFIG = {
  interval: env.RPC_RANK_INTERVAL,
  sampleCount: env.RPC_RANK_SAMPLE_COUNT,
  timeout: env.RPC_RANK_TIMEOUT,
};

// ============================================================================
// CIRCUIT BREAKER & RUNTIME HEALTH MONITORING
// ============================================================================
// These provide runtime resilience against RPC provider failures.
// Circuit breakers prevent repeated calls to failing providers.
// Runtime health monitoring continuously checks provider health.

const CIRCUIT_BREAKER_CONFIG: CircuitBreakerConfig = {
  failureThreshold: env.CIRCUIT_BREAKER_FAILURE_THRESHOLD,
  resetTimeoutMs: env.CIRCUIT_BREAKER_RESET_TIMEOUT_MS,
  halfOpenSuccessThreshold: env.CIRCUIT_BREAKER_HALF_OPEN_SUCCESS_THRESHOLD,
};

// Global circuit breaker manager (shared across all chains)
const circuitBreakerManager = new CircuitBreakerManager(CIRCUIT_BREAKER_CONFIG);

// Reputation store for persistence (singleton)
let reputationStore: ReputationStore | null = null;

// Initialize reputation store and connect to circuit breaker
async function initializeReputationStore(): Promise<void> {
  if (!env.DATABASE_URL) {
    configLogger.warn({}, "No DATABASE_URL configured, reputation persistence disabled");
    return;
  }

  reputationStore = getReputationStore({
    databaseUrl: env.DATABASE_URL,
    flushIntervalMs: 5 * 60 * 1000, // 5 minutes
    debugLogging: env.PONDER_LOG_LEVEL === "debug",
  });

  await reputationStore.initialize();

  // Connect circuit breaker to reputation store for persistence
  circuitBreakerManager.setOnChange((providerName, stats) => {
    if (reputationStore) {
      // Extract chain info from provider name (format: "chainId:providerName")
      let chainId = 0;
      let chainName = "unknown";
      let provider = providerName;

      if (providerName.includes(":")) {
        const parts = providerName.split(":");
        chainId = parseInt(parts[0] ?? "0", 10) || 0;
        chainName = parts[0] ?? "unknown";
        provider = parts[1] ?? providerName;
      }

      reputationStore.updateFromCircuitBreaker(chainId, chainName, provider, stats);
    }
  });

  configLogger.info({}, "ReputationStore initialized and connected to CircuitBreakerManager");
}

// Start reputation store initialization (non-blocking)
initializeReputationStore().catch((error) => {
  configLogger.error(
    { error: error instanceof Error ? error.message : String(error) },
    "Failed to initialize ReputationStore"
  );
});

// Global quota tracker manager (shared across all chains)
const quotaTrackerManager = getQuotaManager({
  alchemy: {
    dailyLimit: env.RPC_QUOTA_ALCHEMY_DAILY,
    monthlyLimit: env.RPC_QUOTA_ALCHEMY_MONTHLY,
    warningThreshold: env.RPC_QUOTA_WARNING_THRESHOLD,
    criticalThreshold: env.RPC_QUOTA_CRITICAL_THRESHOLD,
  },
  infura: {
    dailyLimit: env.RPC_QUOTA_INFURA_DAILY,
    monthlyLimit: env.RPC_QUOTA_INFURA_MONTHLY,
    warningThreshold: env.RPC_QUOTA_WARNING_THRESHOLD,
    criticalThreshold: env.RPC_QUOTA_CRITICAL_THRESHOLD,
  },
  quiknode: {
    dailyLimit: env.RPC_QUOTA_QUIKNODE_DAILY,
    monthlyLimit: env.RPC_QUOTA_QUIKNODE_MONTHLY,
    warningThreshold: env.RPC_QUOTA_WARNING_THRESHOLD,
    criticalThreshold: env.RPC_QUOTA_CRITICAL_THRESHOLD,
  },
  ankr: {
    dailyLimit: env.RPC_QUOTA_ANKR_DAILY,
    monthlyLimit: env.RPC_QUOTA_ANKR_MONTHLY,
    warningThreshold: env.RPC_QUOTA_WARNING_THRESHOLD,
    criticalThreshold: env.RPC_QUOTA_CRITICAL_THRESHOLD,
  },
  publicnode: {
    dailyLimit: env.RPC_QUOTA_PUBLICNODE_DAILY,
    monthlyLimit: env.RPC_QUOTA_PUBLICNODE_MONTHLY,
    warningThreshold: env.RPC_QUOTA_WARNING_THRESHOLD,
    criticalThreshold: env.RPC_QUOTA_CRITICAL_THRESHOLD,
  },
  llamanodes: {
    dailyLimit: env.RPC_QUOTA_LLAMANODES_DAILY,
    monthlyLimit: env.RPC_QUOTA_LLAMANODES_MONTHLY,
    warningThreshold: env.RPC_QUOTA_WARNING_THRESHOLD,
    criticalThreshold: env.RPC_QUOTA_CRITICAL_THRESHOLD,
  },
});

// Log quota tracking status at startup
if (env.RPC_QUOTA_TRACKING_ENABLED) {
  configLogger.info(
    { providers: Object.keys(quotaTrackerManager.getAllStatus()) },
    "Quota tracking enabled for RPC providers"
  );
}

// Runtime health monitor (started after config initialization)
let runtimeHealthMonitor: RuntimeHealthMonitor | null = null;

/**
 * Get RPC URLs for a chain from validated environment
 */
function getRpcUrls(chainPrefix: string): {
  alchemy?: string;
  infura?: string;
  quiknode?: string;
  ankr?: string;
  publicnode?: string;
  llamanodes?: string;
  legacy?: string;
} {
  const envKey = (suffix: string) => `${chainPrefix}_${suffix}` as keyof EnvConfig;

  return {
    alchemy: env[envKey("RPC_ALCHEMY")] as string | undefined,
    infura: env[envKey("RPC_INFURA")] as string | undefined,
    quiknode: env[envKey("RPC_QUIKNODE")] as string | undefined,
    ankr: env[envKey("RPC_ANKR")] as string | undefined,
    publicnode: env[envKey("RPC_PUBLICNODE")] as string | undefined,
    llamanodes: env[envKey("RPC_LLAMANODES")] as string | undefined,
    legacy: env[envKey("RPC_URL")] as string | undefined,
  };
}

/**
 * Run health checks and filter out unhealthy providers
 * Returns healthy providers only
 *
 * @param chainPrefix - Environment variable prefix (e.g., "ETHEREUM_SEPOLIA")
 * @returns Promise<Record<string, string>> - Map of healthy provider names to URLs
 */
async function getHealthyProviders(chainPrefix: string): Promise<Record<string, string>> {
  const urls = getRpcUrls(chainPrefix);

  // Build provider map for health checks
  const providerMap: Record<string, string> = {};
  if (urls.alchemy) providerMap["alchemy"] = urls.alchemy;
  if (urls.infura) providerMap["infura"] = urls.infura;
  if (urls.quiknode) providerMap["quiknode"] = urls.quiknode;
  if (urls.ankr) providerMap["ankr"] = urls.ankr;
  if (urls.publicnode) providerMap["publicnode"] = urls.publicnode;
  if (urls.llamanodes) providerMap["llamanodes"] = urls.llamanodes;

  if (Object.keys(providerMap).length === 0) {
    return {};
  }

  // HEALTH CHECK: Test all providers
  logHealthCheckStart(chainPrefix, Object.keys(providerMap));
  const healthCheckResults = await healthCheckProviders(providerMap);

  // Log individual results
  for (const result of healthCheckResults) {
    logHealthCheckResult(
      chainPrefix,
      result.provider,
      result.success,
      result.latency,
      result.error
    );
  }

  // Summary
  const passed = healthCheckResults.filter((r) => r.success).length;
  const failed = healthCheckResults.filter((r) => !r.success).length;
  logHealthCheckSummary(chainPrefix, passed, failed, healthCheckResults.length);

  // Return only healthy providers
  const healthyProviders: Record<string, string> = {};
  for (const result of healthCheckResults) {
    if (result.success) {
      healthyProviders[result.provider] = result.url;
    }
  }

  return healthyProviders;
}

/**
 * Creates a resilient transport with failover, load balancing, and smart ranking.
 *
 * NOW WITH HEALTH-CHECK INTEGRATION:
 * - Only creates transports for providers that passed health checks
 * - Eliminates SSL/connection errors by filtering bad providers upfront
 *
 * Architecture with healthy providers:
 * - fallback() with rank: automatically promotes faster/more stable providers
 * - loadBalance() for Primary Pool and Fallback Pool (if 4+ providers)
 * - rateLimit() for each individual provider
 *
 * @param chainPrefix - Environment variable prefix (e.g., "ETHEREUM_SEPOLIA")
 * @param healthyProviders - Map of healthy provider names (from health check)
 * @returns Transport or null if no healthy RPC providers
 */
function createResilientTransport(
  chainPrefix: string,
  healthyProviders: Record<string, string>
): Transport | null {
  const urls = getRpcUrls(chainPrefix);

  // Create rate-limited transports ONLY for healthy providers
  const transports: Transport[] = [];

  // Tier 1: Free/Public providers (preferred - no quota limits)
  if (urls.publicnode && healthyProviders["publicnode"]) {
    transports.push(
      rateLimit(http(urls.publicnode), { requestsPerSecond: RATE_LIMITS.publicnode })
    );
  }
  if (urls.llamanodes && healthyProviders["llamanodes"]) {
    transports.push(
      rateLimit(http(urls.llamanodes), { requestsPerSecond: RATE_LIMITS.llamanodes })
    );
  }
  if (urls.ankr && healthyProviders["ankr"]) {
    transports.push(
      rateLimit(http(urls.ankr), { requestsPerSecond: RATE_LIMITS.ankr })
    );
  }

  // Tier 2: Freemium providers (have quota limits)
  if (urls.alchemy && healthyProviders["alchemy"]) {
    transports.push(
      rateLimit(http(urls.alchemy), { requestsPerSecond: RATE_LIMITS.alchemy })
    );
  }
  if (urls.infura && healthyProviders["infura"]) {
    transports.push(
      rateLimit(http(urls.infura), { requestsPerSecond: RATE_LIMITS.infura })
    );
  }
  if (urls.quiknode && healthyProviders["quiknode"]) {
    transports.push(
      rateLimit(http(urls.quiknode), { requestsPerSecond: RATE_LIMITS.quiknode })
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
  const address = (value || "0x0000000000000000000000000000000000000000") as `0x${string}`;
  // Debug: log contract address resolution
  configLogger.info({
    envKey: key,
    hasValue: !!value,
    address: address.slice(0, 10) + "...",
  }, `Contract address resolved for ${key}`);
  return address;
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

// Build networks dynamically with async health checks
// Using async IIFE pattern because Ponder doesn't support async config
async function buildNetworksWithHealthChecks() {
  const configuredNetworks: Record<
    string,
    { chainId: number; transport: Transport }
  > = {};
  const enabledNetworkKeys: string[] = [];

  // Run health checks for all networks in parallel
  const healthCheckPromises = Object.entries(networkEnvPrefixes).map(
    async ([networkKey, envPrefix]) => {
      const healthyProviders = await getHealthyProviders(envPrefix);
      return { networkKey, envPrefix, healthyProviders };
    }
  );

  const healthCheckResults = await Promise.all(healthCheckPromises);

  // Build transports with healthy providers only
  for (const { networkKey, envPrefix, healthyProviders } of healthCheckResults) {
    const transport = createResilientTransport(envPrefix, healthyProviders);
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

  // Start runtime health monitor for all healthy providers
  const allProviderUrls: Record<string, string> = {};
  for (const { envPrefix, healthyProviders } of healthCheckResults) {
    for (const [providerName, url] of Object.entries(healthyProviders)) {
      // Use unique key combining chain prefix and provider name
      allProviderUrls[`${envPrefix}_${providerName}`] = url;
    }
  }

  if (Object.keys(allProviderUrls).length > 0) {
    runtimeHealthMonitor = createRuntimeHealthMonitor(
      allProviderUrls,
      {
        checkIntervalMs: env.RUNTIME_HEALTH_CHECK_INTERVAL_MS,
        timeoutMs: env.RUNTIME_HEALTH_CHECK_TIMEOUT_MS,
        failureThreshold: env.RUNTIME_HEALTH_FAILURE_THRESHOLD,
        debugLogging: env.RUNTIME_HEALTH_DEBUG_LOGGING,
      },
      circuitBreakerManager
    );

    configLogger.info({
      providers: Object.keys(allProviderUrls),
      checkIntervalMs: env.RUNTIME_HEALTH_CHECK_INTERVAL_MS,
    }, "Runtime health monitor started");
  }

  return { configuredNetworks, enabledNetworkKeys };
}

// Execute async setup
const { configuredNetworks, enabledNetworkKeys } = await buildNetworksWithHealthChecks();

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

  // Debug: log what we're processing
  configLogger.info({
    networkKey,
    hasContractAddrs: !!contractAddrs,
    identityAddr: contractAddrs?.["identity"]?.slice(0, 10),
    startBlock,
  }, `Processing contracts for ${networkKey}`);

  // Skip if no contract addresses for this network
  if (!contractAddrs) continue;

  // Identity Registry (ERC1967 Proxy + Implementation)
  configuredContracts[`IdentityRegistry${pascalNetworkKey}`] = {
    network: networkKey,
    abi: mergeAbis([ERC1967ProxyAbi, IdentityRegistryAbi]),
    address: contractAddrs["identity"]!,
    startBlock,
  };

  // Reputation Registry (ERC1967 Proxy + Implementation)
  configuredContracts[`ReputationRegistry${pascalNetworkKey}`] = {
    network: networkKey,
    abi: mergeAbis([ERC1967ProxyAbi, ReputationRegistryAbi]),
    address: contractAddrs["reputation"]!,
    startBlock,
  };

  // Validation Registry (ERC1967 Proxy + Implementation)
  configuredContracts[`ValidationRegistry${pascalNetworkKey}`] = {
    network: networkKey,
    abi: mergeAbis([ERC1967ProxyAbi, ValidationRegistryAbi]),
    address: contractAddrs["validation"]!,
    startBlock,
  };
}

// ============================================================================
// DATABASE RESILIENCE INITIALIZATION
// ============================================================================
// Initialize health monitoring and Dead Letter Queue for additional resilience

async function initializeDatabaseResilience(databaseUrl: string): Promise<void> {
  try {
    // Start database health monitor (separate connection for health checks)
    await createDatabaseHealthMonitor(databaseUrl, {
      checkIntervalMs: 30_000, // Check every 30 seconds
      latencyWarningThresholdMs: 100,
      latencyCriticalThresholdMs: 500,
      maxConnectionRetries: 5,
      debugLogging: false,
    });

    // Initialize Dead Letter Queue for failed events
    // Note: eventProcessor is not provided here - DLQ stores events for manual review
    // In the future, we can add a retry processor that re-inserts events via Ponder
    await createDeadLetterQueue(databaseUrl, {
      retryIntervalMs: 5 * 60 * 1000, // 5 minutes
      maxRetries: 3,
      maxQueueSize: 1000,
      autoRetry: false, // Manual retry only - events need to go through Ponder
      debugLogging: false,
    });

    configLogger.info({}, "Database resilience features initialized");
  } catch (error) {
    // Don't throw - these are optional features
    configLogger.warn(
      { error: error instanceof Error ? error.message : String(error) },
      "Failed to initialize database resilience features (non-fatal)"
    );
  }
}

// Initialize database resilience (fire and forget - don't block Ponder startup)
initializeDatabaseResilience(env.DATABASE_URL).catch((error) => {
  configLogger.error(
    { error: error instanceof Error ? error.message : String(error) },
    "Database resilience initialization failed"
  );
});

// ============================================================================
// GRACEFUL SHUTDOWN
// ============================================================================
// Stop all monitors and cleanup on process exit

async function gracefulShutdown(signal: string): Promise<void> {
  configLogger.info({ signal }, `Received ${signal}, shutting down gracefully`);

  // Stop runtime health monitor
  if (runtimeHealthMonitor) {
    runtimeHealthMonitor.stop();
  }

  // Stop database health monitor
  await stopDatabaseHealthMonitor();

  // Shutdown DLQ
  await shutdownDeadLetterQueue();

  // Shutdown reputation store (flushes pending data)
  if (reputationStore) {
    await reputationStore.shutdown();
  }

  configLogger.info({}, "All monitors stopped, exiting");
}

process.on('SIGINT', () => {
  gracefulShutdown('SIGINT').catch(console.error);
});

process.on('SIGTERM', () => {
  gracefulShutdown('SIGTERM').catch(console.error);
});

// ============================================================================
// DATABASE SCHEMA CONFIGURATION
// ============================================================================
// Ponder uses a dedicated 'ponder' schema to isolate blockchain events from
// application data. Blockchain events are public and immutable.

/**
 * Build database connection string with schema configuration
 *
 * Adds PostgreSQL options to set search_path to the 'ponder' schema.
 * This ensures all Ponder tables (Event, Checkpoint) are created in the
 * ponder schema rather than the public schema.
 */
function buildDatabaseUrlWithSchema(baseUrl: string, schema: string = "ponder"): string {
  const url = new URL(baseUrl);

  // Get existing options or create new ones
  const existingOptions = url.searchParams.get("options") || "";

  // Build the search_path option
  // Format: -c search_path=ponder,public (ponder first, then public for extensions)
  const searchPathOption = `-c search_path=${schema},public`;

  // Combine with existing options if present
  const newOptions = existingOptions
    ? `${existingOptions} ${searchPathOption}`
    : searchPathOption;

  url.searchParams.set("options", newOptions);

  return url.toString();
}

// ============================================================================
// PONDER CONFIGURATION
// ============================================================================

export default createConfig({
  // Database configuration - uses PostgreSQL from validated env with ponder schema
  database: {
    kind: "postgres",
    connectionString: buildDatabaseUrlWithSchema(env.DATABASE_URL),
  },

  // Network configurations (dynamically built)
  networks: configuredNetworks,

  // Contract configurations (dynamically built)
  contracts: configuredContracts,
});
