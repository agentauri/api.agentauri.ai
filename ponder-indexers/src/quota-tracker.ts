/**
 * Quota Tracker for RPC Providers
 *
 * Tracks daily/monthly request quotas per provider to avoid exhausting free tiers.
 * Automatically rotates to other providers when quota thresholds are reached.
 *
 * Features:
 * - Per-provider quota tracking (daily/monthly)
 * - Warning and critical thresholds
 * - Rate limit header parsing (X-RateLimit-*, Retry-After)
 * - Automatic quota reset detection
 * - Integration with circuit breaker for rate limit errors
 */

import { configLogger } from "./logger";

/**
 * Quota configuration for a single provider
 */
export interface ProviderQuotaConfig {
  /** Daily request limit (0 = unlimited) */
  dailyLimit: number;
  /** Monthly request limit (0 = unlimited) */
  monthlyLimit: number;
  /** Warning threshold as percentage (0.8 = 80%) */
  warningThreshold: number;
  /** Critical threshold as percentage (0.95 = 95%) */
  criticalThreshold: number;
}

/**
 * Default quota configurations per provider
 * Based on free tier limits as of 2024
 */
export const DEFAULT_PROVIDER_QUOTAS: Record<string, ProviderQuotaConfig> = {
  alchemy: {
    dailyLimit: 300_000_000, // 300M compute units/month ~ 10M/day
    monthlyLimit: 300_000_000,
    warningThreshold: 0.8,
    criticalThreshold: 0.95,
  },
  infura: {
    dailyLimit: 100_000, // ~3.3K/day on free tier
    monthlyLimit: 100_000,
    warningThreshold: 0.8,
    criticalThreshold: 0.95,
  },
  quiknode: {
    dailyLimit: 50_000,
    monthlyLimit: 1_500_000,
    warningThreshold: 0.8,
    criticalThreshold: 0.95,
  },
  ankr: {
    dailyLimit: 0, // Unlimited on public tier (rate limited instead)
    monthlyLimit: 0,
    warningThreshold: 0.8,
    criticalThreshold: 0.95,
  },
  publicnode: {
    dailyLimit: 0, // Unlimited (rate limited instead)
    monthlyLimit: 0,
    warningThreshold: 0.8,
    criticalThreshold: 0.95,
  },
  llamanodes: {
    dailyLimit: 0, // Unlimited (rate limited instead)
    monthlyLimit: 0,
    warningThreshold: 0.8,
    criticalThreshold: 0.95,
  },
};

/**
 * Quota usage tracking data
 */
export interface QuotaUsage {
  dailyRequests: number;
  monthlyRequests: number;
  lastDailyReset: Date;
  lastMonthlyReset: Date;
  rateLimitedUntil: Date | null;
  lastRequestTime: Date | null;
}

/**
 * Quota status for a provider
 */
export interface QuotaStatus {
  provider: string;
  canMakeRequest: boolean;
  dailyUsagePercent: number;
  monthlyUsagePercent: number;
  isWarning: boolean;
  isCritical: boolean;
  isRateLimited: boolean;
  rateLimitedUntil: Date | null;
  remainingDaily: number;
  remainingMonthly: number;
}

/**
 * Quota warning event
 */
export interface QuotaWarning {
  provider: string;
  type: "daily_warning" | "daily_critical" | "monthly_warning" | "monthly_critical" | "rate_limited";
  usagePercent: number;
  remaining: number;
  message: string;
}

/**
 * Rate limit info from response headers
 */
export interface RateLimitInfo {
  remaining?: number;
  limit?: number;
  resetTime?: Date;
  retryAfter?: number; // seconds
}

/**
 * Parse rate limit headers from RPC response
 */
export function parseRateLimitHeaders(headers: Headers | Record<string, string>): RateLimitInfo {
  const info: RateLimitInfo = {};

  const getHeader = (name: string): string | null => {
    if (headers instanceof Headers) {
      return headers.get(name);
    }
    return headers[name] ?? headers[name.toLowerCase()] ?? null;
  };

  // Standard rate limit headers
  const remaining = getHeader("X-RateLimit-Remaining") ?? getHeader("x-ratelimit-remaining");
  if (remaining) {
    info.remaining = parseInt(remaining, 10);
  }

  const limit = getHeader("X-RateLimit-Limit") ?? getHeader("x-ratelimit-limit");
  if (limit) {
    info.limit = parseInt(limit, 10);
  }

  const reset = getHeader("X-RateLimit-Reset") ?? getHeader("x-ratelimit-reset");
  if (reset) {
    // Could be Unix timestamp or seconds until reset
    const resetVal = parseInt(reset, 10);
    if (resetVal > 1_000_000_000) {
      // Unix timestamp
      info.resetTime = new Date(resetVal * 1000);
    } else {
      // Seconds until reset
      info.resetTime = new Date(Date.now() + resetVal * 1000);
    }
  }

  // Retry-After header (for 429 responses)
  const retryAfter = getHeader("Retry-After") ?? getHeader("retry-after");
  if (retryAfter) {
    const retryVal = parseInt(retryAfter, 10);
    if (!isNaN(retryVal)) {
      info.retryAfter = retryVal;
    }
  }

  // Alchemy-specific compute units
  const alchemyRemaining = getHeader("X-Compute-Units-Remaining");
  if (alchemyRemaining) {
    info.remaining = parseInt(alchemyRemaining, 10);
  }

  return info;
}

/**
 * Check if an error is a rate limit error (HTTP 429)
 */
export function isRateLimitError(error: unknown): boolean {
  if (error instanceof Error) {
    const message = error.message.toLowerCase();
    return (
      message.includes("429") ||
      message.includes("rate limit") ||
      message.includes("too many requests") ||
      message.includes("quota exceeded")
    );
  }
  return false;
}

/**
 * Default quota config for unknown providers (unlimited, like public nodes)
 */
const DEFAULT_UNLIMITED_QUOTA: ProviderQuotaConfig = {
  dailyLimit: 0,
  monthlyLimit: 0,
  warningThreshold: 0.8,
  criticalThreshold: 0.95,
};

/**
 * Quota Tracker for a single provider
 */
export class ProviderQuotaTracker {
  private usage: QuotaUsage;

  constructor(
    private readonly providerName: string,
    private readonly config: ProviderQuotaConfig = DEFAULT_UNLIMITED_QUOTA
  ) {
    const now = new Date();
    this.usage = {
      dailyRequests: 0,
      monthlyRequests: 0,
      lastDailyReset: this.getStartOfDay(now),
      lastMonthlyReset: this.getStartOfMonth(now),
      rateLimitedUntil: null,
      lastRequestTime: null,
    };
  }

  /**
   * Check if a request can be made (quota not exceeded)
   */
  canMakeRequest(): boolean {
    this.checkAndResetQuotas();

    // Check rate limiting first
    if (this.isRateLimited()) {
      return false;
    }

    // Unlimited quotas
    if (this.config.dailyLimit === 0 && this.config.monthlyLimit === 0) {
      return true;
    }

    // Check critical thresholds
    const dailyUsage = this.getDailyUsagePercent();
    const monthlyUsage = this.getMonthlyUsagePercent();

    return dailyUsage < this.config.criticalThreshold && monthlyUsage < this.config.criticalThreshold;
  }

  /**
   * Record a successful request
   */
  recordRequest(): void {
    this.checkAndResetQuotas();
    this.usage.dailyRequests++;
    this.usage.monthlyRequests++;
    this.usage.lastRequestTime = new Date();
  }

  /**
   * Handle rate limit response (HTTP 429)
   */
  handleRateLimit(retryAfterSeconds?: number): void {
    const retryAfter = retryAfterSeconds ?? 60; // Default 1 minute
    this.usage.rateLimitedUntil = new Date(Date.now() + retryAfter * 1000);

    configLogger.warn(
      {
        provider: this.providerName,
        rateLimitedUntil: this.usage.rateLimitedUntil.toISOString(),
        retryAfterSeconds: retryAfter,
      },
      `Provider ${this.providerName} rate limited, will retry after ${retryAfter}s`
    );
  }

  /**
   * Update quota from rate limit headers
   */
  updateFromHeaders(headers: Headers | Record<string, string>): void {
    const info = parseRateLimitHeaders(headers);

    if (info.retryAfter) {
      this.handleRateLimit(info.retryAfter);
    }

    // Could use remaining info for more accurate tracking
    // but we primarily track our own request counts
  }

  /**
   * Check if currently rate limited
   */
  isRateLimited(): boolean {
    if (!this.usage.rateLimitedUntil) {
      return false;
    }
    if (new Date() >= this.usage.rateLimitedUntil) {
      this.usage.rateLimitedUntil = null;
      return false;
    }
    return true;
  }

  /**
   * Get current quota status
   */
  getStatus(): QuotaStatus {
    this.checkAndResetQuotas();

    const dailyUsage = this.getDailyUsagePercent();
    const monthlyUsage = this.getMonthlyUsagePercent();

    const isWarning =
      dailyUsage >= this.config.warningThreshold || monthlyUsage >= this.config.warningThreshold;
    const isCritical =
      dailyUsage >= this.config.criticalThreshold || monthlyUsage >= this.config.criticalThreshold;

    return {
      provider: this.providerName,
      canMakeRequest: this.canMakeRequest(),
      dailyUsagePercent: dailyUsage,
      monthlyUsagePercent: monthlyUsage,
      isWarning,
      isCritical,
      isRateLimited: this.isRateLimited(),
      rateLimitedUntil: this.usage.rateLimitedUntil,
      remainingDaily:
        this.config.dailyLimit === 0 ? Infinity : this.config.dailyLimit - this.usage.dailyRequests,
      remainingMonthly:
        this.config.monthlyLimit === 0
          ? Infinity
          : this.config.monthlyLimit - this.usage.monthlyRequests,
    };
  }

  /**
   * Check for quota warnings
   */
  checkWarnings(): QuotaWarning[] {
    const warnings: QuotaWarning[] = [];
    const status = this.getStatus();

    if (status.isRateLimited) {
      warnings.push({
        provider: this.providerName,
        type: "rate_limited",
        usagePercent: 100,
        remaining: 0,
        message: `Provider ${this.providerName} is rate limited until ${status.rateLimitedUntil?.toISOString()}`,
      });
    }

    if (this.config.dailyLimit > 0) {
      if (status.dailyUsagePercent >= this.config.criticalThreshold) {
        warnings.push({
          provider: this.providerName,
          type: "daily_critical",
          usagePercent: status.dailyUsagePercent * 100,
          remaining: status.remainingDaily,
          message: `Provider ${this.providerName} daily quota critical: ${(status.dailyUsagePercent * 100).toFixed(1)}% used`,
        });
      } else if (status.dailyUsagePercent >= this.config.warningThreshold) {
        warnings.push({
          provider: this.providerName,
          type: "daily_warning",
          usagePercent: status.dailyUsagePercent * 100,
          remaining: status.remainingDaily,
          message: `Provider ${this.providerName} daily quota warning: ${(status.dailyUsagePercent * 100).toFixed(1)}% used`,
        });
      }
    }

    if (this.config.monthlyLimit > 0) {
      if (status.monthlyUsagePercent >= this.config.criticalThreshold) {
        warnings.push({
          provider: this.providerName,
          type: "monthly_critical",
          usagePercent: status.monthlyUsagePercent * 100,
          remaining: status.remainingMonthly,
          message: `Provider ${this.providerName} monthly quota critical: ${(status.monthlyUsagePercent * 100).toFixed(1)}% used`,
        });
      } else if (status.monthlyUsagePercent >= this.config.warningThreshold) {
        warnings.push({
          provider: this.providerName,
          type: "monthly_warning",
          usagePercent: status.monthlyUsagePercent * 100,
          remaining: status.remainingMonthly,
          message: `Provider ${this.providerName} monthly quota warning: ${(status.monthlyUsagePercent * 100).toFixed(1)}% used`,
        });
      }
    }

    return warnings;
  }

  /**
   * Get raw usage data (for persistence)
   */
  getUsage(): QuotaUsage {
    return { ...this.usage };
  }

  /**
   * Restore usage data (from persistence)
   */
  restoreUsage(usage: Partial<QuotaUsage>): void {
    if (usage.dailyRequests !== undefined) {
      this.usage.dailyRequests = usage.dailyRequests;
    }
    if (usage.monthlyRequests !== undefined) {
      this.usage.monthlyRequests = usage.monthlyRequests;
    }
    if (usage.lastDailyReset) {
      this.usage.lastDailyReset = new Date(usage.lastDailyReset);
    }
    if (usage.lastMonthlyReset) {
      this.usage.lastMonthlyReset = new Date(usage.lastMonthlyReset);
    }
    if (usage.rateLimitedUntil) {
      this.usage.rateLimitedUntil = new Date(usage.rateLimitedUntil);
    }
    this.checkAndResetQuotas();
  }

  /**
   * Reset quotas if day/month has changed
   */
  private checkAndResetQuotas(): void {
    const now = new Date();
    const startOfDay = this.getStartOfDay(now);
    const startOfMonth = this.getStartOfMonth(now);

    // Daily reset
    if (startOfDay > this.usage.lastDailyReset) {
      configLogger.info(
        {
          provider: this.providerName,
          previousUsage: this.usage.dailyRequests,
        },
        `Daily quota reset for ${this.providerName}`
      );
      this.usage.dailyRequests = 0;
      this.usage.lastDailyReset = startOfDay;
    }

    // Monthly reset
    if (startOfMonth > this.usage.lastMonthlyReset) {
      configLogger.info(
        {
          provider: this.providerName,
          previousUsage: this.usage.monthlyRequests,
        },
        `Monthly quota reset for ${this.providerName}`
      );
      this.usage.monthlyRequests = 0;
      this.usage.lastMonthlyReset = startOfMonth;
    }
  }

  private getDailyUsagePercent(): number {
    if (this.config.dailyLimit === 0) return 0;
    return this.usage.dailyRequests / this.config.dailyLimit;
  }

  private getMonthlyUsagePercent(): number {
    if (this.config.monthlyLimit === 0) return 0;
    return this.usage.monthlyRequests / this.config.monthlyLimit;
  }

  private getStartOfDay(date: Date): Date {
    const d = new Date(date);
    d.setUTCHours(0, 0, 0, 0);
    return d;
  }

  private getStartOfMonth(date: Date): Date {
    const d = new Date(date);
    d.setUTCDate(1);
    d.setUTCHours(0, 0, 0, 0);
    return d;
  }
}

/**
 * Manager for multiple provider quota trackers
 */
export class QuotaTrackerManager {
  private trackers = new Map<string, ProviderQuotaTracker>();

  constructor(private readonly customConfigs: Record<string, ProviderQuotaConfig> = {}) {}

  /**
   * Get or create a quota tracker for a provider
   */
  getTracker(providerName: string): ProviderQuotaTracker {
    let tracker = this.trackers.get(providerName);
    if (!tracker) {
      const config = this.customConfigs[providerName] ?? DEFAULT_PROVIDER_QUOTAS[providerName] ?? DEFAULT_UNLIMITED_QUOTA;
      tracker = new ProviderQuotaTracker(providerName, config);
      this.trackers.set(providerName, tracker);
    }
    return tracker;
  }

  /**
   * Check if a provider can accept requests
   */
  canMakeRequest(providerName: string): boolean {
    return this.getTracker(providerName).canMakeRequest();
  }

  /**
   * Record a request for a provider
   */
  recordRequest(providerName: string): void {
    this.getTracker(providerName).recordRequest();
  }

  /**
   * Handle rate limit for a provider
   */
  handleRateLimit(providerName: string, retryAfterSeconds?: number): void {
    this.getTracker(providerName).handleRateLimit(retryAfterSeconds);
  }

  /**
   * Get providers sorted by remaining quota (most quota first)
   */
  getProvidersByQuota(): string[] {
    const statuses: { name: string; score: number }[] = [];

    for (const [name, tracker] of this.trackers) {
      const status = tracker.getStatus();
      if (!status.canMakeRequest) {
        continue;
      }

      // Score based on remaining quota (higher is better)
      // Unlimited providers get max score
      const dailyScore =
        status.remainingDaily === Infinity ? 1 : 1 - status.dailyUsagePercent;
      const monthlyScore =
        status.remainingMonthly === Infinity ? 1 : 1 - status.monthlyUsagePercent;

      statuses.push({
        name,
        score: Math.min(dailyScore, monthlyScore),
      });
    }

    // Sort by score descending (most quota first)
    statuses.sort((a, b) => b.score - a.score);

    return statuses.map((s) => s.name);
  }

  /**
   * Get all quota warnings
   */
  getAllWarnings(): QuotaWarning[] {
    const warnings: QuotaWarning[] = [];
    for (const tracker of this.trackers.values()) {
      warnings.push(...tracker.checkWarnings());
    }
    return warnings;
  }

  /**
   * Get status for all providers
   */
  getAllStatus(): Record<string, QuotaStatus> {
    const status: Record<string, QuotaStatus> = {};
    for (const [name, tracker] of this.trackers) {
      status[name] = tracker.getStatus();
    }
    return status;
  }

  /**
   * Log all warnings (if any)
   */
  logWarnings(): void {
    const warnings = this.getAllWarnings();
    for (const warning of warnings) {
      if (warning.type.includes("critical")) {
        configLogger.error(warning, warning.message);
      } else {
        configLogger.warn(warning, warning.message);
      }
    }
  }
}

// Export singleton instance
let quotaManager: QuotaTrackerManager | null = null;

export function getQuotaManager(
  customConfigs?: Record<string, ProviderQuotaConfig>
): QuotaTrackerManager {
  if (!quotaManager) {
    quotaManager = new QuotaTrackerManager(customConfigs);
  }
  return quotaManager;
}

export function resetQuotaManager(): void {
  quotaManager = null;
}
