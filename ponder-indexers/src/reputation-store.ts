/**
 * Reputation Store for RPC Providers
 *
 * Persists provider reputation data to PostgreSQL for recovery across restarts.
 * Integrates with CircuitBreaker and QuotaTracker for unified state management.
 *
 * Features:
 * - Load reputation state on startup
 * - Periodic flush to database
 * - Recovery of circuit breaker state
 * - Historical metrics tracking
 */

import { Pool } from "pg";
import { configLogger } from "./logger";
import type { CircuitState, CircuitBreakerStats } from "./circuit-breaker";
import type { QuotaUsage } from "./quota-tracker";

/**
 * Provider reputation data stored in database
 */
export interface ProviderReputation {
  chainId: number;
  chainName: string;
  providerName: string;

  // Request metrics
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;

  // Latency metrics (ms)
  avgLatencyMs: number | null;
  minLatencyMs: number | null;
  maxLatencyMs: number | null;
  p50LatencyMs: number | null;
  p95LatencyMs: number | null;
  p99LatencyMs: number | null;

  // Circuit breaker state
  circuitState: CircuitState;
  consecutiveFailures: number;
  lastFailureAt: Date | null;
  lastSuccessAt: Date | null;

  // Quota tracking
  dailyRequests: number;
  monthlyRequests: number;
  dailyQuotaLimit: number | null;
  monthlyQuotaLimit: number | null;
  lastDailyReset: Date;
  lastMonthlyReset: Date;

  // Rate limiting
  rateLimitedUntil: Date | null;
  rateLimitCount: number;

  // Timestamps
  createdAt: Date;
  updatedAt: Date;
}

/**
 * Configuration for reputation store
 */
export interface ReputationStoreConfig {
  /** Database connection string */
  databaseUrl: string;
  /** Flush interval in ms (default: 5 minutes) */
  flushIntervalMs: number;
  /** Enable debug logging */
  debugLogging: boolean;
}

const DEFAULT_CONFIG: ReputationStoreConfig = {
  databaseUrl: "",
  flushIntervalMs: 5 * 60 * 1000, // 5 minutes
  debugLogging: false,
};

/**
 * In-memory cache of provider reputation data
 */
interface ReputationCache {
  reputation: ProviderReputation;
  dirty: boolean; // True if needs to be flushed to DB
}

/**
 * Reputation Store implementation
 */
export class ReputationStore {
  private pool: Pool | null = null;
  private cache = new Map<string, ReputationCache>();
  private flushInterval: NodeJS.Timeout | null = null;
  private config: ReputationStoreConfig;
  private isConnected = false;

  constructor(config: Partial<ReputationStoreConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Initialize the store and load existing reputation data
   */
  async initialize(): Promise<void> {
    if (!this.config.databaseUrl) {
      configLogger.warn({}, "ReputationStore: No database URL configured, running in memory-only mode");
      return;
    }

    try {
      this.pool = new Pool({
        connectionString: this.config.databaseUrl,
        max: 3, // Small pool for periodic writes
        idleTimeoutMillis: 30000,
        connectionTimeoutMillis: 5000,
      });

      // Test connection
      const client = await this.pool.connect();
      await client.query("SELECT 1");
      client.release();
      this.isConnected = true;

      configLogger.info({}, "ReputationStore: Connected to database");

      // Ensure table exists (auto-migration)
      await this.ensureTableExists();

      // Load existing reputation data
      await this.loadFromDatabase();

      // Start periodic flush
      this.startFlushInterval();
    } catch (error) {
      configLogger.error(
        { error: error instanceof Error ? error.message : String(error) },
        "ReputationStore: Failed to connect to database, running in memory-only mode"
      );
      this.isConnected = false;
    }
  }

  /**
   * Ensure the reputation table exists (auto-migration)
   */
  private async ensureTableExists(): Promise<void> {
    if (!this.pool || !this.isConnected) return;

    try {
      await this.pool.query(`
        CREATE TABLE IF NOT EXISTS ponder_provider_reputation (
          id SERIAL PRIMARY KEY,
          chain_id INTEGER NOT NULL,
          chain_name VARCHAR(50) NOT NULL,
          provider_name VARCHAR(50) NOT NULL,
          total_requests BIGINT NOT NULL DEFAULT 0,
          successful_requests BIGINT NOT NULL DEFAULT 0,
          failed_requests BIGINT NOT NULL DEFAULT 0,
          avg_latency_ms DOUBLE PRECISION,
          min_latency_ms DOUBLE PRECISION,
          max_latency_ms DOUBLE PRECISION,
          p50_latency_ms DOUBLE PRECISION,
          p95_latency_ms DOUBLE PRECISION,
          p99_latency_ms DOUBLE PRECISION,
          circuit_state VARCHAR(20) NOT NULL DEFAULT 'closed',
          consecutive_failures INTEGER NOT NULL DEFAULT 0,
          last_failure_at TIMESTAMPTZ,
          last_success_at TIMESTAMPTZ,
          daily_requests INTEGER NOT NULL DEFAULT 0,
          monthly_requests INTEGER NOT NULL DEFAULT 0,
          daily_quota_limit INTEGER,
          monthly_quota_limit INTEGER,
          last_daily_reset TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
          last_monthly_reset TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
          rate_limited_until TIMESTAMPTZ,
          rate_limit_count INTEGER NOT NULL DEFAULT 0,
          created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
          updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
          CONSTRAINT uq_ponder_provider_reputation_chain_provider UNIQUE (chain_id, provider_name)
        )
      `);

      // Create indexes if they don't exist
      await this.pool.query(`
        CREATE INDEX IF NOT EXISTS idx_ponder_provider_reputation_chain_id
          ON ponder_provider_reputation(chain_id)
      `);
      await this.pool.query(`
        CREATE INDEX IF NOT EXISTS idx_ponder_provider_reputation_circuit_state
          ON ponder_provider_reputation(circuit_state)
      `);

      configLogger.info({}, "ReputationStore: Table verified/created successfully");
    } catch (error) {
      configLogger.warn(
        { error: error instanceof Error ? error.message : String(error) },
        "ReputationStore: Failed to ensure table exists (non-fatal)"
      );
    }
  }

  /**
   * Load all reputation data from database
   */
  private async loadFromDatabase(): Promise<void> {
    if (!this.pool || !this.isConnected) return;

    try {
      const result = await this.pool.query(`
        SELECT
          chain_id, chain_name, provider_name,
          total_requests, successful_requests, failed_requests,
          avg_latency_ms, min_latency_ms, max_latency_ms,
          p50_latency_ms, p95_latency_ms, p99_latency_ms,
          circuit_state, consecutive_failures,
          last_failure_at, last_success_at,
          daily_requests, monthly_requests,
          daily_quota_limit, monthly_quota_limit,
          last_daily_reset, last_monthly_reset,
          rate_limited_until, rate_limit_count,
          created_at, updated_at
        FROM ponder_provider_reputation
      `);

      for (const row of result.rows) {
        const key = this.getCacheKey(row.chain_id, row.provider_name);
        const reputation: ProviderReputation = {
          chainId: row.chain_id,
          chainName: row.chain_name,
          providerName: row.provider_name,
          totalRequests: parseInt(row.total_requests, 10),
          successfulRequests: parseInt(row.successful_requests, 10),
          failedRequests: parseInt(row.failed_requests, 10),
          avgLatencyMs: row.avg_latency_ms,
          minLatencyMs: row.min_latency_ms,
          maxLatencyMs: row.max_latency_ms,
          p50LatencyMs: row.p50_latency_ms,
          p95LatencyMs: row.p95_latency_ms,
          p99LatencyMs: row.p99_latency_ms,
          circuitState: row.circuit_state as CircuitState,
          consecutiveFailures: row.consecutive_failures,
          lastFailureAt: row.last_failure_at,
          lastSuccessAt: row.last_success_at,
          dailyRequests: row.daily_requests,
          monthlyRequests: row.monthly_requests,
          dailyQuotaLimit: row.daily_quota_limit,
          monthlyQuotaLimit: row.monthly_quota_limit,
          lastDailyReset: row.last_daily_reset,
          lastMonthlyReset: row.last_monthly_reset,
          rateLimitedUntil: row.rate_limited_until,
          rateLimitCount: row.rate_limit_count,
          createdAt: row.created_at,
          updatedAt: row.updated_at,
        };

        this.cache.set(key, { reputation, dirty: false });
      }

      configLogger.info(
        { count: result.rows.length },
        "ReputationStore: Loaded reputation data from database"
      );
    } catch (error) {
      configLogger.error(
        { error: error instanceof Error ? error.message : String(error) },
        "ReputationStore: Failed to load reputation data"
      );
    }
  }

  /**
   * Get reputation for a provider
   */
  getReputation(chainId: number, providerName: string): ProviderReputation | null {
    const key = this.getCacheKey(chainId, providerName);
    const cached = this.cache.get(key);
    return cached?.reputation ?? null;
  }

  /**
   * Update reputation for a provider
   */
  updateReputation(
    chainId: number,
    chainName: string,
    providerName: string,
    update: Partial<Omit<ProviderReputation, "chainId" | "chainName" | "providerName" | "createdAt" | "updatedAt">>
  ): void {
    const key = this.getCacheKey(chainId, providerName);
    let cached = this.cache.get(key);

    if (!cached) {
      // Create new reputation entry
      const now = new Date();
      cached = {
        reputation: {
          chainId,
          chainName,
          providerName,
          totalRequests: 0,
          successfulRequests: 0,
          failedRequests: 0,
          avgLatencyMs: null,
          minLatencyMs: null,
          maxLatencyMs: null,
          p50LatencyMs: null,
          p95LatencyMs: null,
          p99LatencyMs: null,
          circuitState: "closed",
          consecutiveFailures: 0,
          lastFailureAt: null,
          lastSuccessAt: null,
          dailyRequests: 0,
          monthlyRequests: 0,
          dailyQuotaLimit: null,
          monthlyQuotaLimit: null,
          lastDailyReset: now,
          lastMonthlyReset: now,
          rateLimitedUntil: null,
          rateLimitCount: 0,
          createdAt: now,
          updatedAt: now,
        },
        dirty: true,
      };
      this.cache.set(key, cached);
    }

    // Apply updates
    Object.assign(cached.reputation, update);
    cached.reputation.updatedAt = new Date();
    cached.dirty = true;

    if (this.config.debugLogging) {
      configLogger.debug(
        { chainId, providerName, update },
        "ReputationStore: Updated reputation"
      );
    }
  }

  /**
   * Update from circuit breaker stats
   */
  updateFromCircuitBreaker(
    chainId: number,
    chainName: string,
    providerName: string,
    stats: CircuitBreakerStats
  ): void {
    this.updateReputation(chainId, chainName, providerName, {
      circuitState: stats.state,
      consecutiveFailures: stats.failures,
      totalRequests: stats.totalRequests,
      failedRequests: stats.totalFailures,
      successfulRequests: stats.totalRequests - stats.totalFailures,
      lastFailureAt: stats.lastFailureTime ? new Date(stats.lastFailureTime) : null,
      lastSuccessAt: stats.lastSuccessTime ? new Date(stats.lastSuccessTime) : null,
    });
  }

  /**
   * Update from quota usage
   */
  updateFromQuotaUsage(
    chainId: number,
    chainName: string,
    providerName: string,
    usage: QuotaUsage
  ): void {
    this.updateReputation(chainId, chainName, providerName, {
      dailyRequests: usage.dailyRequests,
      monthlyRequests: usage.monthlyRequests,
      lastDailyReset: usage.lastDailyReset,
      lastMonthlyReset: usage.lastMonthlyReset,
      rateLimitedUntil: usage.rateLimitedUntil,
    });
  }

  /**
   * Record a latency sample
   */
  recordLatency(chainId: number, chainName: string, providerName: string, latencyMs: number): void {
    const key = this.getCacheKey(chainId, providerName);
    const cached = this.cache.get(key);

    if (!cached) {
      this.updateReputation(chainId, chainName, providerName, {
        avgLatencyMs: latencyMs,
        minLatencyMs: latencyMs,
        maxLatencyMs: latencyMs,
      });
      return;
    }

    const rep = cached.reputation;

    // Update min/max
    if (rep.minLatencyMs === null || latencyMs < rep.minLatencyMs) {
      rep.minLatencyMs = latencyMs;
    }
    if (rep.maxLatencyMs === null || latencyMs > rep.maxLatencyMs) {
      rep.maxLatencyMs = latencyMs;
    }

    // Update average (exponential moving average)
    if (rep.avgLatencyMs === null) {
      rep.avgLatencyMs = latencyMs;
    } else {
      const alpha = 0.1; // Smoothing factor
      rep.avgLatencyMs = alpha * latencyMs + (1 - alpha) * rep.avgLatencyMs;
    }

    cached.dirty = true;
  }

  /**
   * Flush dirty entries to database
   */
  async flush(): Promise<void> {
    if (!this.pool || !this.isConnected) return;

    const dirtyEntries = Array.from(this.cache.entries()).filter(([_, v]) => v.dirty);

    if (dirtyEntries.length === 0) {
      if (this.config.debugLogging) {
        configLogger.debug({}, "ReputationStore: No dirty entries to flush");
      }
      return;
    }

    const client = await this.pool.connect();
    try {
      await client.query("BEGIN");

      for (const [, cached] of dirtyEntries) {
        const rep = cached.reputation;

        await client.query(
          `
          INSERT INTO ponder_provider_reputation (
            chain_id, chain_name, provider_name,
            total_requests, successful_requests, failed_requests,
            avg_latency_ms, min_latency_ms, max_latency_ms,
            p50_latency_ms, p95_latency_ms, p99_latency_ms,
            circuit_state, consecutive_failures,
            last_failure_at, last_success_at,
            daily_requests, monthly_requests,
            daily_quota_limit, monthly_quota_limit,
            last_daily_reset, last_monthly_reset,
            rate_limited_until, rate_limit_count
          ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
          ON CONFLICT (chain_id, provider_name) DO UPDATE SET
            chain_name = EXCLUDED.chain_name,
            total_requests = EXCLUDED.total_requests,
            successful_requests = EXCLUDED.successful_requests,
            failed_requests = EXCLUDED.failed_requests,
            avg_latency_ms = EXCLUDED.avg_latency_ms,
            min_latency_ms = EXCLUDED.min_latency_ms,
            max_latency_ms = EXCLUDED.max_latency_ms,
            p50_latency_ms = EXCLUDED.p50_latency_ms,
            p95_latency_ms = EXCLUDED.p95_latency_ms,
            p99_latency_ms = EXCLUDED.p99_latency_ms,
            circuit_state = EXCLUDED.circuit_state,
            consecutive_failures = EXCLUDED.consecutive_failures,
            last_failure_at = EXCLUDED.last_failure_at,
            last_success_at = EXCLUDED.last_success_at,
            daily_requests = EXCLUDED.daily_requests,
            monthly_requests = EXCLUDED.monthly_requests,
            daily_quota_limit = EXCLUDED.daily_quota_limit,
            monthly_quota_limit = EXCLUDED.monthly_quota_limit,
            last_daily_reset = EXCLUDED.last_daily_reset,
            last_monthly_reset = EXCLUDED.last_monthly_reset,
            rate_limited_until = EXCLUDED.rate_limited_until,
            rate_limit_count = EXCLUDED.rate_limit_count,
            updated_at = CURRENT_TIMESTAMP
          `,
          [
            rep.chainId,
            rep.chainName,
            rep.providerName,
            rep.totalRequests,
            rep.successfulRequests,
            rep.failedRequests,
            rep.avgLatencyMs,
            rep.minLatencyMs,
            rep.maxLatencyMs,
            rep.p50LatencyMs,
            rep.p95LatencyMs,
            rep.p99LatencyMs,
            rep.circuitState,
            rep.consecutiveFailures,
            rep.lastFailureAt,
            rep.lastSuccessAt,
            rep.dailyRequests,
            rep.monthlyRequests,
            rep.dailyQuotaLimit,
            rep.monthlyQuotaLimit,
            rep.lastDailyReset,
            rep.lastMonthlyReset,
            rep.rateLimitedUntil,
            rep.rateLimitCount,
          ]
        );

        cached.dirty = false;
      }

      await client.query("COMMIT");

      configLogger.info(
        { count: dirtyEntries.length },
        "ReputationStore: Flushed reputation data to database"
      );
    } catch (error) {
      await client.query("ROLLBACK");
      configLogger.error(
        { error: error instanceof Error ? error.message : String(error) },
        "ReputationStore: Failed to flush reputation data"
      );
    } finally {
      client.release();
    }
  }

  /**
   * Start periodic flush interval
   */
  private startFlushInterval(): void {
    if (this.flushInterval) {
      clearInterval(this.flushInterval);
    }

    this.flushInterval = setInterval(() => {
      this.flush().catch((error) => {
        configLogger.error(
          { error: error instanceof Error ? error.message : String(error) },
          "ReputationStore: Periodic flush failed"
        );
      });
    }, this.config.flushIntervalMs);

    configLogger.info(
      { intervalMs: this.config.flushIntervalMs },
      "ReputationStore: Started periodic flush"
    );
  }

  /**
   * Get all reputation data
   */
  getAllReputation(): ProviderReputation[] {
    return Array.from(this.cache.values()).map((c) => c.reputation);
  }

  /**
   * Shutdown the store
   */
  async shutdown(): Promise<void> {
    if (this.flushInterval) {
      clearInterval(this.flushInterval);
      this.flushInterval = null;
    }

    // Final flush
    await this.flush();

    if (this.pool) {
      await this.pool.end();
      this.pool = null;
    }

    configLogger.info({}, "ReputationStore: Shutdown complete");
  }

  /**
   * Generate cache key
   */
  private getCacheKey(chainId: number, providerName: string): string {
    return `${chainId}:${providerName}`;
  }
}

// Singleton instance
let reputationStore: ReputationStore | null = null;

export function getReputationStore(config?: Partial<ReputationStoreConfig>): ReputationStore {
  if (!reputationStore) {
    reputationStore = new ReputationStore(config);
  }
  return reputationStore;
}

export function resetReputationStore(): void {
  if (reputationStore) {
    reputationStore.shutdown().catch(() => {});
    reputationStore = null;
  }
}
