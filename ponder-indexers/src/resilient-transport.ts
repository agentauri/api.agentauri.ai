/**
 * Resilient Transport Wrapper
 *
 * Creates a fault-tolerant transport layer that wraps multiple RPC providers
 * with circuit breaker protection. Unlike Viem's built-in fallback, this
 * implementation:
 *
 * 1. Uses circuit breakers to avoid repeated failures
 * 2. Integrates with runtime health monitoring
 * 3. Provides detailed logging for debugging
 * 4. Gracefully degrades when all providers fail
 *
 * @see https://github.com/ponder-sh/ponder/issues/861
 */

import { custom, type Transport } from "viem";
import {
  CircuitBreakerManager,
  type CircuitBreakerConfig,
  DEFAULT_CIRCUIT_BREAKER_CONFIG,
} from "./circuit-breaker";
import { configLogger } from "./logger";

export interface ProviderConfig {
  name: string;
  url: string;
  transport: Transport;
  /** Optional priority (lower = higher priority, default: 0) */
  priority?: number;
}

export interface ResilientTransportConfig {
  /** Circuit breaker configuration */
  circuitBreaker?: CircuitBreakerConfig;
  /** Request timeout in ms (default: 30000) */
  requestTimeoutMs?: number;
  /** Maximum retries per request across all providers (default: 3) */
  maxRetries?: number;
  /** Enable detailed request logging (default: false) */
  debugLogging?: boolean;
}

const DEFAULT_RESILIENT_TRANSPORT_CONFIG: Required<ResilientTransportConfig> = {
  circuitBreaker: DEFAULT_CIRCUIT_BREAKER_CONFIG,
  requestTimeoutMs: 30_000,
  maxRetries: 3,
  debugLogging: false,
};

export interface ResilientTransportStats {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  providerStats: Record<
    string,
    {
      requests: number;
      successes: number;
      failures: number;
      circuitState: string;
    }
  >;
}

/**
 * Creates a resilient transport that wraps multiple providers with circuit breakers
 *
 * @param providers - Array of provider configurations
 * @param config - Resilient transport configuration
 * @returns A Viem-compatible transport
 */
export function createResilientTransport(
  providers: ProviderConfig[],
  config: ResilientTransportConfig = {}
): { transport: Transport; getStats: () => ResilientTransportStats } {
  const fullConfig = { ...DEFAULT_RESILIENT_TRANSPORT_CONFIG, ...config };
  const breakerManager = new CircuitBreakerManager(fullConfig.circuitBreaker);

  // Sort providers by priority
  const sortedProviders = [...providers].sort(
    (a, b) => (a.priority ?? 0) - (b.priority ?? 0)
  );

  // Initialize circuit breakers for all providers
  for (const provider of sortedProviders) {
    breakerManager.getBreaker(provider.name);
  }

  // Stats tracking
  let totalRequests = 0;
  let successfulRequests = 0;
  let failedRequests = 0;
  const providerRequestCounts = new Map<string, number>();
  const providerSuccessCounts = new Map<string, number>();
  const providerFailureCounts = new Map<string, number>();

  // Initialize counters
  for (const provider of sortedProviders) {
    providerRequestCounts.set(provider.name, 0);
    providerSuccessCounts.set(provider.name, 0);
    providerFailureCounts.set(provider.name, 0);
  }

  const request = async ({ method, params }: { method: string; params?: readonly unknown[] }): Promise<unknown> => {
    totalRequests++;
    let lastError: Error | null = null;
    let attempts = 0;

    // Try each provider in order, respecting circuit breaker state
    for (const provider of sortedProviders) {
      if (attempts >= fullConfig.maxRetries) {
        break;
      }

      const breaker = breakerManager.getBreaker(provider.name);

      // Skip if circuit is open
      if (!breaker.canRequest()) {
        if (fullConfig.debugLogging) {
          configLogger.debug(
            { provider: provider.name, method },
            `Skipping ${provider.name} - circuit open`
          );
        }
        continue;
      }

      attempts++;
      providerRequestCounts.set(
        provider.name,
        (providerRequestCounts.get(provider.name) ?? 0) + 1
      );

      try {
        if (fullConfig.debugLogging) {
          configLogger.debug(
            { provider: provider.name, method, attempt: attempts },
            `Attempting request via ${provider.name}`
          );
        }

        // Create a timeout wrapper
        const result = await Promise.race([
          provider.transport({ chain: undefined, retryCount: 0 }).request({
            method,
            params,
          }),
          new Promise((_, reject) =>
            setTimeout(
              () => reject(new Error(`Request timeout after ${fullConfig.requestTimeoutMs}ms`)),
              fullConfig.requestTimeoutMs
            )
          ),
        ]);

        // Success!
        breaker.recordSuccess();
        successfulRequests++;
        providerSuccessCounts.set(
          provider.name,
          (providerSuccessCounts.get(provider.name) ?? 0) + 1
        );

        if (fullConfig.debugLogging) {
          configLogger.debug(
            { provider: provider.name, method },
            `Request successful via ${provider.name}`
          );
        }

        return result;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        breaker.recordFailure(lastError);
        providerFailureCounts.set(
          provider.name,
          (providerFailureCounts.get(provider.name) ?? 0) + 1
        );

        configLogger.warn(
          {
            provider: provider.name,
            method,
            attempt: attempts,
            error: lastError.message,
          },
          `Request failed via ${provider.name}, trying next provider`
        );

        // Continue to next provider
      }
    }

    // All providers failed
    failedRequests++;
    const errorMessage = `All ${sortedProviders.length} RPC providers failed after ${attempts} attempts`;
    configLogger.error(
      {
        method,
        attempts,
        providers: sortedProviders.map((p) => p.name),
        lastError: lastError?.message,
        circuitStates: Object.fromEntries(
          sortedProviders.map((p) => [p.name, breakerManager.getBreaker(p.name).getState()])
        ),
      },
      errorMessage
    );

    // Throw the last error or a generic one
    throw lastError ?? new Error(errorMessage);
  };

  const transport = custom({ request });

  const getStats = (): ResilientTransportStats => {
    const providerStats: ResilientTransportStats["providerStats"] = {};
    for (const provider of sortedProviders) {
      const breaker = breakerManager.getBreaker(provider.name);
      providerStats[provider.name] = {
        requests: providerRequestCounts.get(provider.name) ?? 0,
        successes: providerSuccessCounts.get(provider.name) ?? 0,
        failures: providerFailureCounts.get(provider.name) ?? 0,
        circuitState: breaker.getState(),
      };
    }

    return {
      totalRequests,
      successfulRequests,
      failedRequests,
      providerStats,
    };
  };

  return { transport, getStats };
}

/**
 * Creates a resilient transport from URL configurations
 *
 * This is a convenience function that creates transports from URLs
 * and wraps them with circuit breaker protection.
 *
 * @param providers - Map of provider name to URL
 * @param config - Resilient transport configuration
 * @returns A Viem-compatible transport with stats function
 */
export function createResilientTransportFromUrls(
  providers: Record<string, string>,
  httpTransportFactory: (url: string) => Transport,
  config: ResilientTransportConfig = {}
): { transport: Transport; getStats: () => ResilientTransportStats } {
  const providerConfigs: ProviderConfig[] = Object.entries(providers).map(
    ([name, url], index) => ({
      name,
      url,
      transport: httpTransportFactory(url),
      priority: index, // Use insertion order as priority
    })
  );

  return createResilientTransport(providerConfigs, config);
}

/**
 * Log transport stats periodically
 *
 * @param getStats - Stats function from createResilientTransport
 * @param intervalMs - Logging interval in ms (default: 60000)
 * @returns Cleanup function to stop logging
 */
export function startStatsLogging(
  getStats: () => ResilientTransportStats,
  intervalMs = 60_000
): () => void {
  const interval = setInterval(() => {
    const stats = getStats();
    const successRate =
      stats.totalRequests > 0
        ? ((stats.successfulRequests / stats.totalRequests) * 100).toFixed(1)
        : "N/A";

    configLogger.info(
      {
        totalRequests: stats.totalRequests,
        successfulRequests: stats.successfulRequests,
        failedRequests: stats.failedRequests,
        successRate: `${successRate}%`,
        providerStats: stats.providerStats,
      },
      "Resilient transport stats"
    );
  }, intervalMs);

  return () => clearInterval(interval);
}
