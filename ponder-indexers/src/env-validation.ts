/**
 * Environment Variable Validation with Zod
 *
 * Validates all environment variables at startup to ensure the application
 * has valid configuration before processing any events.
 */
import { z } from "zod";

/**
 * HTTPS URL validator - ensures all RPC URLs use secure connections
 */
const httpsUrl = z.string().refine(
  (url) => url.startsWith("https://"),
  { message: "URL must use HTTPS protocol" }
);

/**
 * PostgreSQL connection string validator
 */
const postgresUrl = z.string().refine(
  (url) => url.startsWith("postgresql://") || url.startsWith("postgres://"),
  { message: "DATABASE_URL must be a PostgreSQL connection string" }
);

/**
 * Ethereum address validator (0x-prefixed 40 hex chars)
 */
const ethereumAddress = z.string().regex(
  /^0x[a-fA-F0-9]{40}$/,
  { message: "Invalid Ethereum address format" }
);

/**
 * RPC Provider URLs for each chain (all optional, but at least one chain must be configured)
 */
const rpcProviderSchema = {
  // Testnets
  // Ethereum Sepolia
  ETHEREUM_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_PUBLICNODE: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_LLAMANODES: httpsUrl.optional(),
  ETHEREUM_SEPOLIA_RPC_URL: httpsUrl.optional(), // Legacy fallback

  // Base Sepolia
  BASE_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
  BASE_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
  BASE_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
  BASE_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
  BASE_SEPOLIA_RPC_PUBLICNODE: httpsUrl.optional(),
  BASE_SEPOLIA_RPC_LLAMANODES: httpsUrl.optional(),
  BASE_SEPOLIA_RPC_URL: httpsUrl.optional(),

  // Linea Sepolia
  LINEA_SEPOLIA_RPC_ALCHEMY: httpsUrl.optional(),
  LINEA_SEPOLIA_RPC_INFURA: httpsUrl.optional(),
  LINEA_SEPOLIA_RPC_QUIKNODE: httpsUrl.optional(),
  LINEA_SEPOLIA_RPC_ANKR: httpsUrl.optional(),
  LINEA_SEPOLIA_RPC_PUBLICNODE: httpsUrl.optional(),
  LINEA_SEPOLIA_RPC_LLAMANODES: httpsUrl.optional(),
  LINEA_SEPOLIA_RPC_URL: httpsUrl.optional(),

  // Polygon Amoy
  POLYGON_AMOY_RPC_ALCHEMY: httpsUrl.optional(),
  POLYGON_AMOY_RPC_INFURA: httpsUrl.optional(),
  POLYGON_AMOY_RPC_QUIKNODE: httpsUrl.optional(),
  POLYGON_AMOY_RPC_ANKR: httpsUrl.optional(),
  POLYGON_AMOY_RPC_PUBLICNODE: httpsUrl.optional(),
  POLYGON_AMOY_RPC_LLAMANODES: httpsUrl.optional(),
  POLYGON_AMOY_RPC_URL: httpsUrl.optional(),

  // Mainnets
  // Ethereum Mainnet
  ETHEREUM_MAINNET_RPC_ALCHEMY: httpsUrl.optional(),
  ETHEREUM_MAINNET_RPC_INFURA: httpsUrl.optional(),
  ETHEREUM_MAINNET_RPC_QUIKNODE: httpsUrl.optional(),
  ETHEREUM_MAINNET_RPC_ANKR: httpsUrl.optional(),
  ETHEREUM_MAINNET_RPC_PUBLICNODE: httpsUrl.optional(),
  ETHEREUM_MAINNET_RPC_LLAMANODES: httpsUrl.optional(),
  ETHEREUM_MAINNET_RPC_URL: httpsUrl.optional(),

  // Base Mainnet
  BASE_MAINNET_RPC_ALCHEMY: httpsUrl.optional(),
  BASE_MAINNET_RPC_INFURA: httpsUrl.optional(),
  BASE_MAINNET_RPC_QUIKNODE: httpsUrl.optional(),
  BASE_MAINNET_RPC_ANKR: httpsUrl.optional(),
  BASE_MAINNET_RPC_PUBLICNODE: httpsUrl.optional(),
  BASE_MAINNET_RPC_LLAMANODES: httpsUrl.optional(),
  BASE_MAINNET_RPC_URL: httpsUrl.optional(),

  // Linea Mainnet
  LINEA_MAINNET_RPC_ALCHEMY: httpsUrl.optional(),
  LINEA_MAINNET_RPC_INFURA: httpsUrl.optional(),
  LINEA_MAINNET_RPC_QUIKNODE: httpsUrl.optional(),
  LINEA_MAINNET_RPC_PUBLICNODE: httpsUrl.optional(),
  LINEA_MAINNET_RPC_LLAMANODES: httpsUrl.optional(),
  LINEA_MAINNET_RPC_URL: httpsUrl.optional(),
};

/**
 * Contract addresses for each chain
 */
const contractAddressSchema = {
  // Testnets
  // Ethereum Sepolia
  ETHEREUM_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
  ETHEREUM_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
  ETHEREUM_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
  ETHEREUM_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // Base Sepolia
  BASE_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
  BASE_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
  BASE_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
  BASE_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // Linea Sepolia
  LINEA_SEPOLIA_IDENTITY_ADDRESS: ethereumAddress.optional(),
  LINEA_SEPOLIA_REPUTATION_ADDRESS: ethereumAddress.optional(),
  LINEA_SEPOLIA_VALIDATION_ADDRESS: ethereumAddress.optional(),
  LINEA_SEPOLIA_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // Polygon Amoy
  POLYGON_AMOY_IDENTITY_ADDRESS: ethereumAddress.optional(),
  POLYGON_AMOY_REPUTATION_ADDRESS: ethereumAddress.optional(),
  POLYGON_AMOY_VALIDATION_ADDRESS: ethereumAddress.optional(),
  POLYGON_AMOY_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // Mainnets
  // Ethereum Mainnet
  ETHEREUM_MAINNET_IDENTITY_ADDRESS: ethereumAddress.optional(),
  ETHEREUM_MAINNET_REPUTATION_ADDRESS: ethereumAddress.optional(),
  ETHEREUM_MAINNET_VALIDATION_ADDRESS: ethereumAddress.optional(),
  ETHEREUM_MAINNET_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // Base Mainnet
  BASE_MAINNET_IDENTITY_ADDRESS: ethereumAddress.optional(),
  BASE_MAINNET_REPUTATION_ADDRESS: ethereumAddress.optional(),
  BASE_MAINNET_VALIDATION_ADDRESS: ethereumAddress.optional(),
  BASE_MAINNET_START_BLOCK: z.coerce.number().int().min(0).optional(),

  // Linea Mainnet
  LINEA_MAINNET_IDENTITY_ADDRESS: ethereumAddress.optional(),
  LINEA_MAINNET_REPUTATION_ADDRESS: ethereumAddress.optional(),
  LINEA_MAINNET_VALIDATION_ADDRESS: ethereumAddress.optional(),
  LINEA_MAINNET_START_BLOCK: z.coerce.number().int().min(0).optional(),
};

/**
 * Rate limiting configuration
 */
const rateLimitSchema = {
  RPC_RATE_LIMIT_ALCHEMY: z.coerce.number().int().min(1).max(100).default(25),
  RPC_RATE_LIMIT_INFURA: z.coerce.number().int().min(1).max(100).default(20),
  RPC_RATE_LIMIT_QUIKNODE: z.coerce.number().int().min(1).max(100).default(25),
  RPC_RATE_LIMIT_ANKR: z.coerce.number().int().min(1).max(100).default(30),
  RPC_RATE_LIMIT_PUBLICNODE: z.coerce.number().int().min(1).max(100).default(5),
  RPC_RATE_LIMIT_LLAMANODES: z.coerce.number().int().min(1).max(100).default(5),
};

/**
 * Quota tracking configuration for RPC providers
 * Limits are monthly request counts (0 = unlimited)
 */
const quotaSchema = {
  // Daily quotas (0 = unlimited)
  RPC_QUOTA_ALCHEMY_DAILY: z.coerce.number().int().min(0).default(0), // Unlimited on free tier
  RPC_QUOTA_INFURA_DAILY: z.coerce.number().int().min(0).default(100000),
  RPC_QUOTA_QUIKNODE_DAILY: z.coerce.number().int().min(0).default(50000),
  RPC_QUOTA_ANKR_DAILY: z.coerce.number().int().min(0).default(0), // Unlimited
  RPC_QUOTA_PUBLICNODE_DAILY: z.coerce.number().int().min(0).default(0), // Unlimited
  RPC_QUOTA_LLAMANODES_DAILY: z.coerce.number().int().min(0).default(0), // Unlimited

  // Monthly quotas (0 = unlimited)
  RPC_QUOTA_ALCHEMY_MONTHLY: z.coerce.number().int().min(0).default(300000000), // 300M compute units
  RPC_QUOTA_INFURA_MONTHLY: z.coerce.number().int().min(0).default(100000),
  RPC_QUOTA_QUIKNODE_MONTHLY: z.coerce.number().int().min(0).default(1500000),
  RPC_QUOTA_ANKR_MONTHLY: z.coerce.number().int().min(0).default(0), // Unlimited
  RPC_QUOTA_PUBLICNODE_MONTHLY: z.coerce.number().int().min(0).default(0), // Unlimited
  RPC_QUOTA_LLAMANODES_MONTHLY: z.coerce.number().int().min(0).default(0), // Unlimited

  // Thresholds for warnings/rotation
  RPC_QUOTA_WARNING_THRESHOLD: z.coerce.number().min(0.1).max(1).default(0.8),
  RPC_QUOTA_CRITICAL_THRESHOLD: z.coerce.number().min(0.1).max(1).default(0.95),

  // Enable/disable quota tracking
  RPC_QUOTA_TRACKING_ENABLED: z.coerce.boolean().default(true),
};

/**
 * Ranking/health check configuration
 */
const rankingSchema = {
  RPC_RANK_INTERVAL: z.coerce.number().int().min(1000).max(60000).default(10000),
  RPC_RANK_SAMPLE_COUNT: z.coerce.number().int().min(1).max(100).default(10),
  RPC_RANK_TIMEOUT: z.coerce.number().int().min(500).max(10000).default(2000),
};

/**
 * Circuit breaker configuration for RPC resilience
 */
const circuitBreakerSchema = {
  // Number of failures before opening the circuit
  CIRCUIT_BREAKER_FAILURE_THRESHOLD: z.coerce.number().int().min(1).max(20).default(5),
  // Time in ms before attempting to close the circuit (1 minute default)
  CIRCUIT_BREAKER_RESET_TIMEOUT_MS: z.coerce.number().int().min(10000).max(300000).default(60000),
  // Number of successful requests in half-open state to close circuit
  CIRCUIT_BREAKER_HALF_OPEN_SUCCESS_THRESHOLD: z.coerce.number().int().min(1).max(10).default(3),
};

/**
 * Runtime health monitoring configuration
 */
const runtimeHealthSchema = {
  // Health check interval in ms (30 seconds default)
  RUNTIME_HEALTH_CHECK_INTERVAL_MS: z.coerce.number().int().min(10000).max(120000).default(30000),
  // Request timeout for health checks in ms
  RUNTIME_HEALTH_CHECK_TIMEOUT_MS: z.coerce.number().int().min(5000).max(30000).default(10000),
  // Number of consecutive failures before marking unhealthy
  RUNTIME_HEALTH_FAILURE_THRESHOLD: z.coerce.number().int().min(1).max(10).default(3),
  // Enable detailed debug logging for health monitoring
  RUNTIME_HEALTH_DEBUG_LOGGING: z.coerce.boolean().default(false),
};

/**
 * Resilient transport configuration
 */
const resilientTransportSchema = {
  // Request timeout for RPC calls in ms
  RESILIENT_TRANSPORT_REQUEST_TIMEOUT_MS: z.coerce.number().int().min(5000).max(60000).default(30000),
  // Maximum retries per request across all providers
  RESILIENT_TRANSPORT_MAX_RETRIES: z.coerce.number().int().min(1).max(10).default(3),
  // Enable detailed debug logging
  RESILIENT_TRANSPORT_DEBUG_LOGGING: z.coerce.boolean().default(false),
};

/**
 * Ponder-specific configuration
 */
const ponderSchema = {
  PONDER_LOG_LEVEL: z.enum(["debug", "info", "warn", "error"]).default("info"),
  PONDER_RPC_REQUEST_TIMEOUT: z.coerce.number().int().min(1000).max(120000).default(30000),
};

/**
 * Complete environment schema
 */
const envSchema = z.object({
  // Database (required)
  DATABASE_URL: postgresUrl,

  // RPC Providers
  ...rpcProviderSchema,

  // Contract Addresses
  ...contractAddressSchema,

  // Rate Limits
  ...rateLimitSchema,

  // Quota Tracking
  ...quotaSchema,

  // Ranking Config
  ...rankingSchema,

  // Circuit Breaker Config
  ...circuitBreakerSchema,

  // Runtime Health Monitoring Config
  ...runtimeHealthSchema,

  // Resilient Transport Config
  ...resilientTransportSchema,

  // Ponder Config
  ...ponderSchema,
});

/**
 * Check that at least one chain has RPC configured
 */
function hasAtLeastOneChainConfigured(env: EnvConfig): boolean {
  const chains = [
    // Testnets
    {
      prefix: "ETHEREUM_SEPOLIA",
      hasRpc: Boolean(
        env.ETHEREUM_SEPOLIA_RPC_ALCHEMY ??
        env.ETHEREUM_SEPOLIA_RPC_INFURA ??
        env.ETHEREUM_SEPOLIA_RPC_QUIKNODE ??
        env.ETHEREUM_SEPOLIA_RPC_ANKR ??
        env.ETHEREUM_SEPOLIA_RPC_PUBLICNODE ??
        env.ETHEREUM_SEPOLIA_RPC_LLAMANODES ??
        env.ETHEREUM_SEPOLIA_RPC_URL
      ),
    },
    {
      prefix: "BASE_SEPOLIA",
      hasRpc: Boolean(
        env.BASE_SEPOLIA_RPC_ALCHEMY ??
        env.BASE_SEPOLIA_RPC_INFURA ??
        env.BASE_SEPOLIA_RPC_QUIKNODE ??
        env.BASE_SEPOLIA_RPC_ANKR ??
        env.BASE_SEPOLIA_RPC_PUBLICNODE ??
        env.BASE_SEPOLIA_RPC_LLAMANODES ??
        env.BASE_SEPOLIA_RPC_URL
      ),
    },
    {
      prefix: "LINEA_SEPOLIA",
      hasRpc: Boolean(
        env.LINEA_SEPOLIA_RPC_ALCHEMY ??
        env.LINEA_SEPOLIA_RPC_INFURA ??
        env.LINEA_SEPOLIA_RPC_QUIKNODE ??
        env.LINEA_SEPOLIA_RPC_ANKR ??
        env.LINEA_SEPOLIA_RPC_PUBLICNODE ??
        env.LINEA_SEPOLIA_RPC_LLAMANODES ??
        env.LINEA_SEPOLIA_RPC_URL
      ),
    },
    {
      prefix: "POLYGON_AMOY",
      hasRpc: Boolean(
        env.POLYGON_AMOY_RPC_ALCHEMY ??
        env.POLYGON_AMOY_RPC_INFURA ??
        env.POLYGON_AMOY_RPC_QUIKNODE ??
        env.POLYGON_AMOY_RPC_ANKR ??
        env.POLYGON_AMOY_RPC_PUBLICNODE ??
        env.POLYGON_AMOY_RPC_LLAMANODES ??
        env.POLYGON_AMOY_RPC_URL
      ),
    },
    // Mainnets
    {
      prefix: "ETHEREUM_MAINNET",
      hasRpc: Boolean(
        env.ETHEREUM_MAINNET_RPC_ALCHEMY ??
        env.ETHEREUM_MAINNET_RPC_INFURA ??
        env.ETHEREUM_MAINNET_RPC_QUIKNODE ??
        env.ETHEREUM_MAINNET_RPC_ANKR ??
        env.ETHEREUM_MAINNET_RPC_PUBLICNODE ??
        env.ETHEREUM_MAINNET_RPC_LLAMANODES ??
        env.ETHEREUM_MAINNET_RPC_URL
      ),
    },
    {
      prefix: "BASE_MAINNET",
      hasRpc: Boolean(
        env.BASE_MAINNET_RPC_ALCHEMY ??
        env.BASE_MAINNET_RPC_INFURA ??
        env.BASE_MAINNET_RPC_QUIKNODE ??
        env.BASE_MAINNET_RPC_ANKR ??
        env.BASE_MAINNET_RPC_PUBLICNODE ??
        env.BASE_MAINNET_RPC_LLAMANODES ??
        env.BASE_MAINNET_RPC_URL
      ),
    },
    {
      prefix: "LINEA_MAINNET",
      hasRpc: Boolean(
        env.LINEA_MAINNET_RPC_ALCHEMY ??
        env.LINEA_MAINNET_RPC_INFURA ??
        env.LINEA_MAINNET_RPC_QUIKNODE ??
        env.LINEA_MAINNET_RPC_PUBLICNODE ??
        env.LINEA_MAINNET_RPC_LLAMANODES ??
        env.LINEA_MAINNET_RPC_URL
      ),
    },
  ];

  return chains.some((chain) => chain.hasRpc);
}

/**
 * Get list of configured chains
 */
export function getConfiguredChains(env: EnvConfig): string[] {
  const chains: string[] = [];

  // Testnets
  if (
    env.ETHEREUM_SEPOLIA_RPC_ALCHEMY ??
    env.ETHEREUM_SEPOLIA_RPC_INFURA ??
    env.ETHEREUM_SEPOLIA_RPC_QUIKNODE ??
    env.ETHEREUM_SEPOLIA_RPC_ANKR ??
    env.ETHEREUM_SEPOLIA_RPC_PUBLICNODE ??
    env.ETHEREUM_SEPOLIA_RPC_LLAMANODES ??
    env.ETHEREUM_SEPOLIA_RPC_URL
  ) {
    chains.push("ethereumSepolia");
  }

  if (
    env.BASE_SEPOLIA_RPC_ALCHEMY ??
    env.BASE_SEPOLIA_RPC_INFURA ??
    env.BASE_SEPOLIA_RPC_QUIKNODE ??
    env.BASE_SEPOLIA_RPC_ANKR ??
    env.BASE_SEPOLIA_RPC_PUBLICNODE ??
    env.BASE_SEPOLIA_RPC_LLAMANODES ??
    env.BASE_SEPOLIA_RPC_URL
  ) {
    chains.push("baseSepolia");
  }

  if (
    env.LINEA_SEPOLIA_RPC_ALCHEMY ??
    env.LINEA_SEPOLIA_RPC_INFURA ??
    env.LINEA_SEPOLIA_RPC_QUIKNODE ??
    env.LINEA_SEPOLIA_RPC_ANKR ??
    env.LINEA_SEPOLIA_RPC_PUBLICNODE ??
    env.LINEA_SEPOLIA_RPC_LLAMANODES ??
    env.LINEA_SEPOLIA_RPC_URL
  ) {
    chains.push("lineaSepolia");
  }

  if (
    env.POLYGON_AMOY_RPC_ALCHEMY ??
    env.POLYGON_AMOY_RPC_INFURA ??
    env.POLYGON_AMOY_RPC_QUIKNODE ??
    env.POLYGON_AMOY_RPC_ANKR ??
    env.POLYGON_AMOY_RPC_PUBLICNODE ??
    env.POLYGON_AMOY_RPC_LLAMANODES ??
    env.POLYGON_AMOY_RPC_URL
  ) {
    chains.push("polygonAmoy");
  }

  // Mainnets
  if (
    env.ETHEREUM_MAINNET_RPC_ALCHEMY ??
    env.ETHEREUM_MAINNET_RPC_INFURA ??
    env.ETHEREUM_MAINNET_RPC_QUIKNODE ??
    env.ETHEREUM_MAINNET_RPC_ANKR ??
    env.ETHEREUM_MAINNET_RPC_PUBLICNODE ??
    env.ETHEREUM_MAINNET_RPC_LLAMANODES ??
    env.ETHEREUM_MAINNET_RPC_URL
  ) {
    chains.push("ethereumMainnet");
  }

  if (
    env.BASE_MAINNET_RPC_ALCHEMY ??
    env.BASE_MAINNET_RPC_INFURA ??
    env.BASE_MAINNET_RPC_QUIKNODE ??
    env.BASE_MAINNET_RPC_ANKR ??
    env.BASE_MAINNET_RPC_PUBLICNODE ??
    env.BASE_MAINNET_RPC_LLAMANODES ??
    env.BASE_MAINNET_RPC_URL
  ) {
    chains.push("baseMainnet");
  }

  if (
    env.LINEA_MAINNET_RPC_ALCHEMY ??
    env.LINEA_MAINNET_RPC_INFURA ??
    env.LINEA_MAINNET_RPC_QUIKNODE ??
    env.LINEA_MAINNET_RPC_PUBLICNODE ??
    env.LINEA_MAINNET_RPC_LLAMANODES ??
    env.LINEA_MAINNET_RPC_URL
  ) {
    chains.push("lineaMainnet");
  }

  return chains;
}

/**
 * Parsed and validated environment type
 */
export type EnvConfig = z.infer<typeof envSchema>;

/**
 * Validate environment variables
 * Throws ZodError if validation fails
 */
export function validateEnv(): EnvConfig {
  const result = envSchema.safeParse(process.env);

  if (!result.success) {
    const errors = result.error.issues.map((issue) => {
      return `  - ${issue.path.join(".")}: ${issue.message}`;
    });

    throw new Error(
      `Environment validation failed:\n${errors.join("\n")}\n\n` +
      `Please check your .env.local file and ensure all required variables are set correctly.`
    );
  }

  // Check that at least one chain is configured
  if (!hasAtLeastOneChainConfigured(result.data)) {
    throw new Error(
      "At least one blockchain network must have an RPC provider configured.\n" +
      "Testnets:\n" +
      "  - ETHEREUM_SEPOLIA_RPC_ALCHEMY (or other providers)\n" +
      "  - BASE_SEPOLIA_RPC_ALCHEMY (or other providers)\n" +
      "  - LINEA_SEPOLIA_RPC_ALCHEMY (or other providers)\n" +
      "  - POLYGON_AMOY_RPC_ALCHEMY (or other providers)\n" +
      "Mainnets:\n" +
      "  - ETHEREUM_MAINNET_RPC_ALCHEMY (or other providers)\n" +
      "  - BASE_MAINNET_RPC_ALCHEMY (or other providers)\n" +
      "  - LINEA_MAINNET_RPC_ALCHEMY (or other providers)"
    );
  }

  return result.data;
}

/**
 * Get validated environment (singleton)
 * Call this at application startup
 */
let cachedEnv: EnvConfig | null = null;

export function getEnv(): EnvConfig {
  if (!cachedEnv) {
    cachedEnv = validateEnv();
  }
  return cachedEnv;
}

/**
 * Reset cached environment (for testing)
 */
export function resetEnvCache(): void {
  cachedEnv = null;
}

export { envSchema };
