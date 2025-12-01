/**
 * RPC Provider Health Check Module
 *
 * Performs preventive health checks on RPC providers at startup to detect
 * SSL errors, connectivity issues, and other problems BEFORE they cause
 * runtime failures during blockchain sync.
 *
 * This eliminates the need for complex runtime error handling and provides
 * clear visibility into which providers are functioning correctly.
 */

import { createPublicClient, http } from "viem";
import { healthCheckLogger } from "./logger";

/**
 * Health check result for a single RPC provider
 */
export interface HealthCheckResult {
  provider: string;
  url: string;
  success: boolean;
  latency?: number; // milliseconds
  error?: string;
}

/**
 * Configuration for health check behavior
 */
interface HealthCheckConfig {
  timeout: number; // milliseconds
  retries: number;
  retryDelay: number; // milliseconds
}

const DEFAULT_CONFIG: HealthCheckConfig = {
  timeout: 2000, // 2 seconds
  retries: 1, // one retry for transient network issues
  retryDelay: 500, // 500ms between retries
};

/**
 * Sleep utility for retry delays
 */
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Perform health check on a single RPC provider
 *
 * Tests the provider by making a simple eth_blockNumber request.
 * This validates:
 * - SSL/TLS connection establishment
 * - Network connectivity
 * - RPC endpoint responsiveness
 * - Basic authentication (if required)
 *
 * @param provider - Human-readable provider name (e.g., "Alchemy", "Infura")
 * @param url - RPC endpoint URL
 * @param config - Health check configuration (timeout, retries)
 * @returns Health check result with success status, latency, and error details
 */
export async function healthCheckProvider(
  provider: string,
  url: string,
  config: HealthCheckConfig = DEFAULT_CONFIG
): Promise<HealthCheckResult> {
  let lastError: Error | undefined;

  // Try the health check with configured number of retries
  for (let attempt = 0; attempt <= config.retries; attempt++) {
    if (attempt > 0) {
      healthCheckLogger.debug({
        provider,
        attempt,
        retryDelay: config.retryDelay,
      }, "Retrying health check after failure");
      await sleep(config.retryDelay);
    }

    try {
      const startTime = Date.now();

      // Create a temporary client for health check
      const client = createPublicClient({
        transport: http(url, {
          timeout: config.timeout,
          // Disable retries at the transport level since we handle retries here
          retryCount: 0,
        }),
      });

      // Test 1: get current block number
      // This validates: SSL → TCP → HTTP → RPC → Response parsing
      const blockNumber = await client.getBlockNumber();

      // Test 2: get block by "latest" tag (what Ponder uses in realtime sync)
      // This catches providers that support eth_blockNumber but fail on eth_getBlockByNumber("latest")
      const latestBlock = await client.getBlock({ blockTag: "latest" });

      // Validate that latest block exists and has expected structure
      if (!latestBlock || !latestBlock.number) {
        throw new Error("Latest block returned null or invalid structure");
      }

      // Sanity check: latest block number should be >= blockNumber
      if (latestBlock.number < blockNumber) {
        throw new Error(
          `Latest block number (${latestBlock.number}) is less than eth_blockNumber (${blockNumber})`
        );
      }

      const latency = Date.now() - startTime;

      return {
        provider,
        url,
        success: true,
        latency,
      };
    } catch (error) {
      lastError = error as Error;

      // Log detailed error info for debugging
      healthCheckLogger.debug({
        provider,
        attempt,
        error: error instanceof Error ? error.message : String(error),
        errorName: error instanceof Error ? error.constructor.name : undefined,
      }, "Health check attempt failed");
    }
  }

  // All retries exhausted - health check failed
  const errorMessage = lastError?.message || "Unknown error";

  return {
    provider,
    url,
    success: false,
    error: errorMessage,
  };
}

/**
 * Perform health checks on multiple RPC providers in parallel
 *
 * @param providers - Map of provider names to URLs
 * @param config - Health check configuration
 * @returns Array of health check results
 */
export async function healthCheckProviders(
  providers: Record<string, string>,
  config?: HealthCheckConfig
): Promise<HealthCheckResult[]> {
  const providerEntries = Object.entries(providers);

  if (providerEntries.length === 0) {
    return [];
  }

  healthCheckLogger.info({
    providerCount: providerEntries.length,
    providers: providerEntries.map(([name]) => name),
  }, "Starting RPC provider health checks");

  // Run all health checks in parallel for speed
  const results = await Promise.all(
    providerEntries.map(([provider, url]) =>
      healthCheckProvider(provider, url, config)
    )
  );

  // Log summary
  const passed = results.filter((r) => r.success).length;
  const failed = results.filter((r) => !r.success).length;

  healthCheckLogger.info({
    total: results.length,
    passed,
    failed,
    results: results.map((r) => ({
      provider: r.provider,
      success: r.success,
      latency: r.latency,
      error: r.error,
    })),
  }, "Health check summary");

  return results;
}

/**
 * Filter providers based on health check results
 *
 * Returns only the URLs of providers that passed the health check.
 *
 * @param providers - Map of provider names to URLs
 * @param healthCheckResults - Results from health check
 * @returns Map of healthy provider names to URLs
 */
export function filterHealthyProviders(
  providers: Record<string, string>,
  healthCheckResults: HealthCheckResult[]
): Record<string, string> {
  const healthyProviders: Record<string, string> = {};

  for (const result of healthCheckResults) {
    if (result.success) {
      healthyProviders[result.provider] = result.url;
    }
  }

  return healthyProviders;
}
