/**
 * Structured Logger with Pino
 *
 * Provides structured JSON logging for the Ponder indexers.
 * Replaces console.log to avoid exposing sensitive infrastructure details.
 */
import pino from "pino";

/**
 * Log level from environment
 */
const LOG_LEVEL = process.env["PONDER_LOG_LEVEL"] ?? "info";

/**
 * Determine if we're in development mode
 */
const isDev = process.env["NODE_ENV"] !== "production";

/**
 * Create the base logger
 * In development, use pino-pretty for readable output
 * In production, use JSON format for log aggregation
 */
export const logger = pino({
  name: "ponder-indexers",
  level: LOG_LEVEL,
  ...(isDev && {
    transport: {
      target: "pino-pretty",
      options: {
        colorize: true,
        translateTime: "SYS:standard",
        ignore: "pid,hostname",
      },
    },
  }),
  // Redact sensitive fields from logs
  redact: {
    paths: [
      "*.password",
      "*.apiKey",
      "*.secret",
      "*.token",
      "DATABASE_URL",
      "*.RPC_ALCHEMY",
      "*.RPC_INFURA",
      "*.RPC_QUIKNODE",
      "*.RPC_ANKR",
    ],
    censor: "[REDACTED]",
  },
});

/**
 * RPC Logger - for RPC transport configuration
 * Masks sensitive provider URLs
 */
export const rpcLogger = logger.child({ component: "rpc" });

/**
 * Log RPC configuration without exposing URLs
 */
export function logRpcConfig(chainPrefix: string, mode: string, providerCount?: number): void {
  rpcLogger.info({
    chain: chainPrefix,
    mode,
    ...(providerCount !== undefined && { providers: providerCount }),
  }, `RPC configured: ${mode}`);
}

/**
 * Log skipped chain
 */
export function logRpcSkipped(chainPrefix: string): void {
  rpcLogger.debug({
    chain: chainPrefix,
  }, "Chain skipped (no RPC configured)");
}

/**
 * Event Handler Logger - for event processing
 */
export const eventLogger = logger.child({ component: "events" });

/**
 * Log event processing
 */
export function logEventProcessed(
  registry: string,
  eventType: string,
  chainId: bigint,
  blockNumber: bigint,
  agentId?: bigint
): void {
  eventLogger.info({
    registry,
    eventType,
    chainId: chainId.toString(),
    blockNumber: blockNumber.toString(),
    ...(agentId !== undefined && { agentId: agentId.toString() }),
  }, `Event processed: ${registry}.${eventType}`);
}

/**
 * Log event processing error
 */
export function logEventError(
  registry: string,
  eventType: string,
  chainId: bigint,
  error: Error
): void {
  eventLogger.error({
    registry,
    eventType,
    chainId: chainId.toString(),
    error: {
      message: error.message,
      stack: error.stack,
    },
  }, `Event processing failed: ${registry}.${eventType}`);
}

/**
 * Config Logger - for configuration loading
 */
export const configLogger = logger.child({ component: "config" });

/**
 * Log config validation
 */
export function logConfigValidated(configuredChains: string[]): void {
  configLogger.info({
    chains: configuredChains,
    chainCount: configuredChains.length,
  }, "Configuration validated successfully");
}

/**
 * Log config error
 */
export function logConfigError(error: Error): void {
  configLogger.error({
    error: {
      message: error.message,
    },
  }, "Configuration validation failed");
}

/**
 * Database Logger - for database operations
 */
export const dbLogger = logger.child({ component: "database" });

/**
 * Log database connection
 */
export function logDbConnected(): void {
  dbLogger.info("Database connected");
}

/**
 * Log checkpoint update
 */
export function logCheckpointUpdated(chainId: bigint, blockNumber: bigint): void {
  dbLogger.debug({
    chainId: chainId.toString(),
    blockNumber: blockNumber.toString(),
  }, "Checkpoint updated");
}

/**
 * Health Check Logger - for RPC provider health checks
 */
export const healthCheckLogger = logger.child({ component: "health-check" });

/**
 * Log health check start
 */
export function logHealthCheckStart(chain: string, providers: string[]): void {
  healthCheckLogger.info({
    chain,
    providers,
    providerCount: providers.length,
  }, `Starting health checks for ${chain}`);
}

/**
 * Log individual health check result
 */
export function logHealthCheckResult(
  chain: string,
  provider: string,
  success: boolean,
  latency?: number,
  error?: string
): void {
  if (success) {
    healthCheckLogger.info({
      chain,
      provider,
      latency,
      status: "OK",
    }, `✅ ${provider}: OK (${latency}ms)`);
  } else {
    healthCheckLogger.warn({
      chain,
      provider,
      error,
      status: "FAILED",
    }, `❌ ${provider}: FAILED - ${error}`);
  }
}

/**
 * Log health check summary
 */
export function logHealthCheckSummary(
  chain: string,
  passed: number,
  failed: number,
  total: number
): void {
  const status = failed === 0 ? "all providers healthy" : `${failed} provider(s) failed`;
  healthCheckLogger.info({
    chain,
    passed,
    failed,
    total,
  }, `Health check complete: ${passed}/${total} providers available (${status})`);
}

export default logger;
