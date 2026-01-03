/**
 * Database Health Monitor for Ponder
 *
 * Provides continuous monitoring of database connectivity and latency
 * using a separate connection pool from Ponder's internal connection.
 *
 * Features:
 * - Periodic health checks (configurable interval)
 * - Latency monitoring with warning thresholds
 * - Connection retry with exponential backoff
 * - Graceful degradation on failure
 */

import { Pool, type PoolConfig, type PoolClient } from "pg";
import { configLogger } from "../logger";

// ============================================================================
// TYPES
// ============================================================================

export interface DatabaseHealthConfig {
  /** Health check interval in milliseconds (default: 30000 = 30s) */
  checkIntervalMs?: number;
  /** Latency warning threshold in milliseconds (default: 100ms) */
  latencyWarningThresholdMs?: number;
  /** Latency critical threshold in milliseconds (default: 500ms) */
  latencyCriticalThresholdMs?: number;
  /** Max connection retries on startup (default: 5) */
  maxConnectionRetries?: number;
  /** Base delay for exponential backoff in ms (default: 1000) */
  baseRetryDelayMs?: number;
  /** Enable debug logging (default: false) */
  debugLogging?: boolean;
}

export interface HealthCheckResult {
  healthy: boolean;
  latencyMs: number;
  timestamp: Date;
  error?: string;
}

export interface DatabaseHealthStats {
  totalChecks: number;
  successfulChecks: number;
  failedChecks: number;
  averageLatencyMs: number;
  lastCheckResult: HealthCheckResult | null;
  consecutiveFailures: number;
}

// ============================================================================
// DEFAULT CONFIGURATION
// ============================================================================

const DEFAULT_CONFIG: Required<DatabaseHealthConfig> = {
  checkIntervalMs: 30_000, // 30 seconds
  latencyWarningThresholdMs: 100,
  latencyCriticalThresholdMs: 500,
  maxConnectionRetries: 5,
  baseRetryDelayMs: 1_000,
  debugLogging: false,
};

// ============================================================================
// DATABASE HEALTH MONITOR
// ============================================================================

export class DatabaseHealthMonitor {
  private pool: Pool | null = null;
  private config: Required<DatabaseHealthConfig>;
  private connectionString: string;
  private checkInterval: NodeJS.Timeout | null = null;
  private stats: DatabaseHealthStats;
  private latencyHistory: number[] = [];
  private readonly maxLatencyHistorySize = 100;

  constructor(connectionString: string, config: DatabaseHealthConfig = {}) {
    this.connectionString = connectionString;
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.stats = {
      totalChecks: 0,
      successfulChecks: 0,
      failedChecks: 0,
      averageLatencyMs: 0,
      lastCheckResult: null,
      consecutiveFailures: 0,
    };
  }

  /**
   * Start the health monitor with retry logic for initial connection
   */
  async start(): Promise<void> {
    await this.connectWithRetry();
    this.startHealthChecks();
    configLogger.info(
      {
        checkIntervalMs: this.config.checkIntervalMs,
        latencyWarningMs: this.config.latencyWarningThresholdMs,
      },
      "Database health monitor started"
    );
  }

  /**
   * Stop the health monitor and close the connection pool
   */
  async stop(): Promise<void> {
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
    }
    if (this.pool) {
      await this.pool.end();
      this.pool = null;
    }
    configLogger.info({}, "Database health monitor stopped");
  }

  /**
   * Get current health statistics
   */
  getStats(): DatabaseHealthStats {
    return { ...this.stats };
  }

  /**
   * Perform a single health check (can be called manually)
   */
  async checkHealth(): Promise<HealthCheckResult> {
    if (!this.pool) {
      return {
        healthy: false,
        latencyMs: 0,
        timestamp: new Date(),
        error: "Connection pool not initialized",
      };
    }

    const startTime = Date.now();
    let client: PoolClient | null = null;

    try {
      client = await this.pool.connect();
      await client.query("SELECT 1");
      const latencyMs = Date.now() - startTime;

      const result: HealthCheckResult = {
        healthy: true,
        latencyMs,
        timestamp: new Date(),
      };

      this.recordSuccess(latencyMs);
      this.logLatencyStatus(latencyMs);

      return result;
    } catch (error) {
      const latencyMs = Date.now() - startTime;
      const errorMessage =
        error instanceof Error ? error.message : String(error);

      const result: HealthCheckResult = {
        healthy: false,
        latencyMs,
        timestamp: new Date(),
        error: errorMessage,
      };

      this.recordFailure(errorMessage);

      return result;
    } finally {
      if (client) {
        client.release();
      }
    }
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  private async connectWithRetry(): Promise<void> {
    let lastError: Error | null = null;

    for (let attempt = 1; attempt <= this.config.maxConnectionRetries; attempt++) {
      try {
        const poolConfig: PoolConfig = {
          connectionString: this.connectionString,
          max: 2, // Small pool for health checks only
          idleTimeoutMillis: 30_000,
          connectionTimeoutMillis: 5_000,
        };

        this.pool = new Pool(poolConfig);

        // Test the connection
        const client = await this.pool.connect();
        await client.query("SELECT 1");
        client.release();

        configLogger.info(
          { attempt },
          "Database health monitor connected successfully"
        );
        return;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        const delay = this.config.baseRetryDelayMs * Math.pow(2, attempt - 1);

        configLogger.warn(
          {
            attempt,
            maxAttempts: this.config.maxConnectionRetries,
            nextRetryMs: delay,
            error: lastError.message,
          },
          "Database health monitor connection failed, retrying..."
        );

        if (attempt < this.config.maxConnectionRetries) {
          await this.sleep(delay);
        }
      }
    }

    configLogger.error(
      {
        maxAttempts: this.config.maxConnectionRetries,
        error: lastError?.message,
      },
      "Database health monitor failed to connect after all retries"
    );

    // Don't throw - allow Ponder to continue even if health monitor fails
    // Health checks will return unhealthy status
  }

  private startHealthChecks(): void {
    this.checkInterval = setInterval(() => {
      void this.checkHealth();
    }, this.config.checkIntervalMs);

    // Perform initial check immediately
    this.checkHealth().catch((error) => {
      configLogger.error({ error }, "Initial health check failed");
    });
  }

  private recordSuccess(latencyMs: number): void {
    this.stats.totalChecks++;
    this.stats.successfulChecks++;
    this.stats.consecutiveFailures = 0;

    // Update latency history
    this.latencyHistory.push(latencyMs);
    if (this.latencyHistory.length > this.maxLatencyHistorySize) {
      this.latencyHistory.shift();
    }

    // Calculate average
    this.stats.averageLatencyMs =
      this.latencyHistory.reduce((sum, l) => sum + l, 0) /
      this.latencyHistory.length;

    this.stats.lastCheckResult = {
      healthy: true,
      latencyMs,
      timestamp: new Date(),
    };

    if (this.config.debugLogging) {
      configLogger.debug(
        { latencyMs, avgLatencyMs: this.stats.averageLatencyMs },
        "Database health check passed"
      );
    }
  }

  private recordFailure(error: string): void {
    this.stats.totalChecks++;
    this.stats.failedChecks++;
    this.stats.consecutiveFailures++;

    this.stats.lastCheckResult = {
      healthy: false,
      latencyMs: 0,
      timestamp: new Date(),
      error,
    };

    configLogger.error(
      {
        consecutiveFailures: this.stats.consecutiveFailures,
        error,
      },
      "Database health check failed"
    );
  }

  private logLatencyStatus(latencyMs: number): void {
    if (latencyMs >= this.config.latencyCriticalThresholdMs) {
      configLogger.error(
        {
          latencyMs,
          threshold: this.config.latencyCriticalThresholdMs,
        },
        "CRITICAL: Database latency exceeds critical threshold"
      );
    } else if (latencyMs >= this.config.latencyWarningThresholdMs) {
      configLogger.warn(
        {
          latencyMs,
          threshold: this.config.latencyWarningThresholdMs,
        },
        "WARNING: Database latency exceeds warning threshold"
      );
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

// ============================================================================
// SINGLETON INSTANCE
// ============================================================================

let healthMonitorInstance: DatabaseHealthMonitor | null = null;

/**
 * Create and start the database health monitor singleton
 */
export async function createDatabaseHealthMonitor(
  connectionString: string,
  config?: DatabaseHealthConfig
): Promise<DatabaseHealthMonitor> {
  if (healthMonitorInstance) {
    configLogger.warn({}, "Database health monitor already exists, reusing");
    return healthMonitorInstance;
  }

  healthMonitorInstance = new DatabaseHealthMonitor(connectionString, config);
  await healthMonitorInstance.start();
  return healthMonitorInstance;
}

/**
 * Get the existing health monitor instance
 */
export function getDatabaseHealthMonitor(): DatabaseHealthMonitor | null {
  return healthMonitorInstance;
}

/**
 * Stop and clean up the health monitor singleton
 */
export async function stopDatabaseHealthMonitor(): Promise<void> {
  if (healthMonitorInstance) {
    await healthMonitorInstance.stop();
    healthMonitorInstance = null;
  }
}
