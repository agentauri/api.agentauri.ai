/**
 * Runtime Health Monitor
 *
 * Continuously monitors RPC provider health at runtime, not just at startup.
 * This complements the startup health checks in health-check.ts by detecting
 * providers that become unhealthy after Ponder starts.
 *
 * Key features:
 * - Periodic health checks (configurable interval)
 * - Integration with circuit breaker to open circuits for unhealthy providers
 * - Detailed logging and stats for debugging
 * - Graceful handling of transient failures
 */

import { configLogger } from "./logger";
import type { CircuitBreakerManager } from "./circuit-breaker";
import type { ReputationStore } from "./reputation-store";

export interface HealthCheckResult {
  provider: string;
  url: string;
  healthy: boolean;
  latencyMs: number | null;
  error?: string;
  blockNumber?: number;
}

export interface RuntimeHealthMonitorConfig {
  /** Health check interval in ms (default: 30000) */
  checkIntervalMs: number;
  /** Request timeout in ms (default: 10000) */
  timeoutMs: number;
  /** Number of consecutive failures before marking unhealthy (default: 3) */
  failureThreshold: number;
  /** Enable detailed logging (default: false) */
  debugLogging: boolean;
}

export const DEFAULT_RUNTIME_HEALTH_CONFIG: RuntimeHealthMonitorConfig = {
  checkIntervalMs: 30_000, // 30 seconds
  timeoutMs: 10_000, // 10 seconds
  failureThreshold: 3,
  debugLogging: false,
};

interface ProviderState {
  url: string;
  consecutiveFailures: number;
  lastCheckTime: number | null;
  lastHealthy: boolean;
  totalChecks: number;
  totalFailures: number;
}

/**
 * Runtime health monitor for RPC providers
 */
export class RuntimeHealthMonitor {
  private intervalId: NodeJS.Timeout | null = null;
  private providerStates = new Map<string, ProviderState>();
  private readonly config: RuntimeHealthMonitorConfig;
  private circuitBreakerManager: CircuitBreakerManager | null = null;
  private reputationStore: ReputationStore | null = null;

  constructor(config: Partial<RuntimeHealthMonitorConfig> = {}) {
    this.config = { ...DEFAULT_RUNTIME_HEALTH_CONFIG, ...config };
  }

  /**
   * Register providers to monitor
   *
   * @param providers - Map of provider name to URL
   */
  registerProviders(providers: Record<string, string>): void {
    for (const [name, url] of Object.entries(providers)) {
      if (!this.providerStates.has(name)) {
        this.providerStates.set(name, {
          url,
          consecutiveFailures: 0,
          lastCheckTime: null,
          lastHealthy: true,
          totalChecks: 0,
          totalFailures: 0,
        });
      }
    }
  }

  /**
   * Link with circuit breaker manager for automatic circuit opening
   *
   * @param manager - Circuit breaker manager instance
   */
  linkCircuitBreakerManager(manager: CircuitBreakerManager): void {
    this.circuitBreakerManager = manager;
  }

  /**
   * Link with reputation store for latency and health tracking
   *
   * @param store - Reputation store instance
   */
  linkReputationStore(store: ReputationStore): void {
    this.reputationStore = store;
  }

  /**
   * Start the health monitoring loop
   */
  start(): void {
    if (this.intervalId) {
      configLogger.warn({}, "Runtime health monitor already running");
      return;
    }

    configLogger.info(
      {
        checkIntervalMs: this.config.checkIntervalMs,
        timeoutMs: this.config.timeoutMs,
        failureThreshold: this.config.failureThreshold,
        providers: Array.from(this.providerStates.keys()),
      },
      "Starting runtime health monitor"
    );

    // Run initial check immediately
    this.runHealthChecks().catch((error) => {
      configLogger.error({ error: String(error) }, "Initial health check failed");
    });

    // Schedule periodic checks
    this.intervalId = setInterval(() => {
      this.runHealthChecks().catch((error) => {
        configLogger.error({ error: String(error) }, "Periodic health check failed");
      });
    }, this.config.checkIntervalMs);
  }

  /**
   * Stop the health monitoring loop
   */
  stop(): void {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
      configLogger.info({}, "Runtime health monitor stopped");
    }
  }

  /**
   * Check if a provider is currently healthy
   *
   * @param providerName - Name of the provider
   * @returns true if healthy, false if unhealthy
   */
  isHealthy(providerName: string): boolean {
    const state = this.providerStates.get(providerName);
    return state?.lastHealthy ?? true; // Assume healthy if unknown
  }

  /**
   * Get health status for all providers
   */
  getHealthStatus(): Record<string, { healthy: boolean; consecutiveFailures: number }> {
    const status: Record<string, { healthy: boolean; consecutiveFailures: number }> = {};
    for (const [name, state] of this.providerStates) {
      status[name] = {
        healthy: state.lastHealthy,
        consecutiveFailures: state.consecutiveFailures,
      };
    }
    return status;
  }

  /**
   * Get detailed stats for all providers
   */
  getStats(): Record<string, ProviderState> {
    const stats: Record<string, ProviderState> = {};
    for (const [name, state] of this.providerStates) {
      stats[name] = { ...state };
    }
    return stats;
  }

  /**
   * Run health checks for all registered providers
   */
  private async runHealthChecks(): Promise<HealthCheckResult[]> {
    const results: HealthCheckResult[] = [];
    const checkPromises: Promise<HealthCheckResult>[] = [];

    for (const [name, state] of this.providerStates) {
      checkPromises.push(this.checkProvider(name, state.url));
    }

    const checkResults = await Promise.allSettled(checkPromises);

    for (const result of checkResults) {
      if (result.status === "fulfilled") {
        results.push(result.value);
        this.updateProviderState(result.value);
      } else {
        configLogger.error(
          { error: result.reason },
          "Health check promise rejected unexpectedly"
        );
      }
    }

    // Log summary
    const healthy = results.filter((r) => r.healthy).length;
    const unhealthy = results.length - healthy;

    if (unhealthy > 0 || this.config.debugLogging) {
      configLogger.info(
        {
          healthy,
          unhealthy,
          results: results.map((r) => ({
            provider: r.provider,
            healthy: r.healthy,
            latencyMs: r.latencyMs,
            error: r.error,
          })),
        },
        `Runtime health check complete: ${healthy}/${results.length} healthy`
      );
    }

    return results;
  }

  /**
   * Check a single provider's health
   */
  private async checkProvider(name: string, url: string): Promise<HealthCheckResult> {
    const startTime = Date.now();

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.config.timeoutMs);

      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "eth_blockNumber",
          params: [],
        }),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const json = (await response.json()) as { result?: string; error?: { message: string } };
      const latencyMs = Date.now() - startTime;

      if (json.error) {
        throw new Error(`RPC error: ${json.error.message}`);
      }

      const blockNumber = json.result ? parseInt(json.result, 16) : undefined;

      if (this.config.debugLogging) {
        configLogger.debug(
          { provider: name, latencyMs, blockNumber },
          `Health check passed for ${name}`
        );
      }

      return {
        provider: name,
        url,
        healthy: true,
        latencyMs,
        blockNumber,
      };
    } catch (error) {
      const latencyMs = Date.now() - startTime;
      const errorMessage = error instanceof Error ? error.message : String(error);

      configLogger.warn(
        { provider: name, latencyMs, error: errorMessage },
        `Health check failed for ${name}`
      );

      return {
        provider: name,
        url,
        healthy: false,
        latencyMs,
        error: errorMessage,
      };
    }
  }

  /**
   * Update provider state based on health check result
   */
  private updateProviderState(result: HealthCheckResult): void {
    const state = this.providerStates.get(result.provider);
    if (!state) return;

    state.lastCheckTime = Date.now();
    state.totalChecks++;

    // Extract chain info from provider name (format: "CHAIN_NAME_providerName")
    const { chainId, chainName, providerName } = this.parseProviderName(result.provider);

    if (result.healthy) {
      // Provider is healthy - reset consecutive failures
      if (state.consecutiveFailures > 0) {
        configLogger.info(
          {
            provider: result.provider,
            previousFailures: state.consecutiveFailures,
          },
          `Provider ${result.provider} recovered after ${state.consecutiveFailures} failures`
        );
      }
      state.consecutiveFailures = 0;
      state.lastHealthy = true;

      // Notify circuit breaker of success
      if (this.circuitBreakerManager) {
        const breaker = this.circuitBreakerManager.getBreaker(result.provider);
        breaker.recordSuccess();
      }

      // Update reputation store with latency
      if (this.reputationStore && result.latencyMs !== null) {
        this.reputationStore.recordLatency(chainId, chainName, providerName, result.latencyMs);
        this.reputationStore.updateReputation(chainId, chainName, providerName, {
          lastSuccessAt: new Date(),
          circuitState: "closed",
        });
      }
    } else {
      // Provider failed - increment failure count
      state.consecutiveFailures++;
      state.totalFailures++;

      // Update reputation store with failure
      if (this.reputationStore) {
        this.reputationStore.updateReputation(chainId, chainName, providerName, {
          lastFailureAt: new Date(),
          consecutiveFailures: state.consecutiveFailures,
        });
      }

      // Check if we've hit the failure threshold
      if (state.consecutiveFailures >= this.config.failureThreshold) {
        if (state.lastHealthy) {
          configLogger.error(
            {
              provider: result.provider,
              consecutiveFailures: state.consecutiveFailures,
              threshold: this.config.failureThreshold,
              error: result.error,
            },
            `Provider ${result.provider} marked UNHEALTHY after ${state.consecutiveFailures} consecutive failures`
          );
        }
        state.lastHealthy = false;

        // Update reputation store with open circuit
        if (this.reputationStore) {
          this.reputationStore.updateReputation(chainId, chainName, providerName, {
            circuitState: "open",
          });
        }

        // Notify circuit breaker
        if (this.circuitBreakerManager) {
          const breaker = this.circuitBreakerManager.getBreaker(result.provider);
          breaker.recordFailure(new Error(result.error ?? "Health check failed"));
        }
      }
    }
  }

  /**
   * Parse provider name to extract chain info
   * Format: "CHAIN_NAME_providerName" (e.g., "ETHEREUM_SEPOLIA_ankr")
   */
  private parseProviderName(fullName: string): { chainId: number; chainName: string; providerName: string } {
    // Chain ID mapping
    const chainIdMap: Record<string, number> = {
      ETHEREUM_SEPOLIA: 11155111,
      BASE_SEPOLIA: 84532,
      LINEA_SEPOLIA: 59141,
      ETHEREUM_MAINNET: 1,
      BASE_MAINNET: 8453,
      LINEA_MAINNET: 59144,
    };

    // Try to extract chain name from provider name
    for (const [chainKey, id] of Object.entries(chainIdMap)) {
      if (fullName.startsWith(chainKey + "_")) {
        const providerName = fullName.slice(chainKey.length + 1);
        return {
          chainId: id,
          chainName: chainKey,
          providerName,
        };
      }
    }

    // Fallback: use full name as provider name
    return {
      chainId: 0,
      chainName: "unknown",
      providerName: fullName,
    };
  }

  /**
   * Force a health check for a specific provider
   *
   * @param providerName - Name of the provider to check
   * @returns Health check result
   */
  async forceCheck(providerName: string): Promise<HealthCheckResult | null> {
    const state = this.providerStates.get(providerName);
    if (!state) {
      configLogger.warn({ provider: providerName }, "Unknown provider for force check");
      return null;
    }

    const result = await this.checkProvider(providerName, state.url);
    this.updateProviderState(result);
    return result;
  }
}

/**
 * Create and start a runtime health monitor
 *
 * @param providers - Map of provider name to URL
 * @param config - Monitor configuration
 * @param circuitBreakerManager - Optional circuit breaker manager to link
 * @returns The running health monitor instance
 */
export function createRuntimeHealthMonitor(
  providers: Record<string, string>,
  config: Partial<RuntimeHealthMonitorConfig> = {},
  circuitBreakerManager?: CircuitBreakerManager
): RuntimeHealthMonitor {
  const monitor = new RuntimeHealthMonitor(config);
  monitor.registerProviders(providers);

  if (circuitBreakerManager) {
    monitor.linkCircuitBreakerManager(circuitBreakerManager);
  }

  monitor.start();
  return monitor;
}
