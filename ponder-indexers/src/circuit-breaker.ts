/**
 * Circuit Breaker Pattern Implementation
 *
 * Provides fault tolerance for RPC providers by tracking failures and
 * preventing requests to unhealthy providers until they recover.
 *
 * States:
 * - CLOSED: Normal operation, requests flow through
 * - OPEN: Provider is unhealthy, requests are blocked
 * - HALF_OPEN: Testing if provider has recovered
 *
 * @see https://martinfowler.com/bliki/CircuitBreaker.html
 */

import { configLogger } from "./logger";

export type CircuitState = "closed" | "open" | "half-open";

/**
 * Callback for circuit breaker state changes (used for persistence)
 */
export type CircuitBreakerOnChange = (providerName: string, stats: CircuitBreakerStats) => void;

/**
 * Persisted circuit breaker state for recovery
 */
export interface PersistedCircuitState {
  state: CircuitState;
  failures: number;
  lastFailureTime: number | null;
  lastSuccessTime: number | null;
  totalRequests: number;
  totalFailures: number;
}

export interface CircuitBreakerConfig {
  /** Number of failures before opening the circuit (default: 5) */
  failureThreshold: number;
  /** Time in ms before attempting to close the circuit (default: 60000) */
  resetTimeoutMs: number;
  /** Number of successful requests in half-open state to close circuit (default: 3) */
  halfOpenSuccessThreshold: number;
}

export const DEFAULT_CIRCUIT_BREAKER_CONFIG: CircuitBreakerConfig = {
  failureThreshold: 5,
  resetTimeoutMs: 60_000, // 1 minute
  halfOpenSuccessThreshold: 3,
};

export interface CircuitBreakerStats {
  state: CircuitState;
  failures: number;
  successes: number;
  lastFailureTime: number | null;
  lastSuccessTime: number | null;
  totalRequests: number;
  totalFailures: number;
}

/**
 * Circuit Breaker implementation for individual RPC providers
 */
export class CircuitBreaker {
  private state: CircuitState = "closed";
  private failures = 0;
  private halfOpenSuccesses = 0;
  private lastFailureTime: number | null = null;
  private lastSuccessTime: number | null = null;
  private totalRequests = 0;
  private totalFailures = 0;
  private onChange?: CircuitBreakerOnChange;

  constructor(
    private readonly providerName: string,
    private readonly config: CircuitBreakerConfig = DEFAULT_CIRCUIT_BREAKER_CONFIG
  ) {}

  /**
   * Set callback for state changes (used for persistence)
   */
  setOnChange(callback: CircuitBreakerOnChange): void {
    this.onChange = callback;
  }

  /**
   * Restore state from persisted data (called on startup)
   */
  restoreState(persisted: PersistedCircuitState): void {
    // Check if the persisted open state has timed out
    if (persisted.state === "open" && persisted.lastFailureTime !== null) {
      const timeSinceFailure = Date.now() - persisted.lastFailureTime;
      if (timeSinceFailure >= this.config.resetTimeoutMs) {
        // Circuit should have reset by now, start in half-open
        this.state = "half-open";
        this.failures = 0;
        configLogger.info(
          { provider: this.providerName, timeSinceFailure },
          `Circuit breaker restored to half-open (timeout expired) for ${this.providerName}`
        );
      } else {
        // Still within timeout, restore as open
        this.state = persisted.state;
        this.failures = persisted.failures;
      }
    } else {
      this.state = persisted.state;
      this.failures = persisted.failures;
    }

    this.lastFailureTime = persisted.lastFailureTime;
    this.lastSuccessTime = persisted.lastSuccessTime;
    this.totalRequests = persisted.totalRequests;
    this.totalFailures = persisted.totalFailures;
    this.halfOpenSuccesses = 0; // Always reset half-open counter

    configLogger.info(
      {
        provider: this.providerName,
        restoredState: this.state,
        failures: this.failures,
        totalRequests: this.totalRequests,
      },
      `Circuit breaker state restored for ${this.providerName}`
    );
  }

  /**
   * Check if a request can be made through this circuit
   *
   * @returns true if the request should proceed, false if it should be blocked
   */
  canRequest(): boolean {
    this.totalRequests++;

    switch (this.state) {
      case "closed":
        return true;

      case "open":
        // Check if enough time has passed to try again
        if (this.shouldAttemptReset()) {
          this.transitionTo("half-open");
          return true;
        }
        return false;

      case "half-open":
        // Allow requests in half-open state to test recovery
        return true;

      default:
        return true;
    }
  }

  /**
   * Record a successful request
   */
  recordSuccess(): void {
    this.lastSuccessTime = Date.now();

    switch (this.state) {
      case "closed":
        // Reset failure count on success
        this.failures = 0;
        break;

      case "half-open":
        this.halfOpenSuccesses++;
        if (this.halfOpenSuccesses >= this.config.halfOpenSuccessThreshold) {
          this.transitionTo("closed");
        }
        break;

      case "open":
        // Shouldn't happen, but handle gracefully
        this.transitionTo("half-open");
        break;
    }
  }

  /**
   * Record a failed request
   */
  recordFailure(error?: Error): void {
    this.lastFailureTime = Date.now();
    this.totalFailures++;

    switch (this.state) {
      case "closed":
        this.failures++;
        if (this.failures >= this.config.failureThreshold) {
          this.transitionTo("open");
        }
        break;

      case "half-open":
        // Any failure in half-open state reopens the circuit
        this.transitionTo("open");
        break;

      case "open":
        // Already open, just update failure time
        break;
    }

    configLogger.warn(
      {
        provider: this.providerName,
        state: this.state,
        failures: this.failures,
        threshold: this.config.failureThreshold,
        error: error?.message,
      },
      `Circuit breaker recorded failure for ${this.providerName}`
    );
  }

  /**
   * Get current circuit breaker state and stats
   */
  getStats(): CircuitBreakerStats {
    return {
      state: this.state,
      failures: this.failures,
      successes: this.halfOpenSuccesses,
      lastFailureTime: this.lastFailureTime,
      lastSuccessTime: this.lastSuccessTime,
      totalRequests: this.totalRequests,
      totalFailures: this.totalFailures,
    };
  }

  /**
   * Get current state
   */
  getState(): CircuitState {
    return this.state;
  }

  /**
   * Force reset the circuit breaker to closed state
   * Use with caution - mainly for testing or manual intervention
   */
  reset(): void {
    this.state = "closed";
    this.failures = 0;
    this.halfOpenSuccesses = 0;
    configLogger.info(
      { provider: this.providerName },
      `Circuit breaker manually reset for ${this.providerName}`
    );
  }

  /**
   * Check if enough time has passed to attempt reset
   */
  private shouldAttemptReset(): boolean {
    if (this.lastFailureTime === null) {
      return true;
    }
    return Date.now() - this.lastFailureTime >= this.config.resetTimeoutMs;
  }

  /**
   * Transition to a new state with logging
   */
  private transitionTo(newState: CircuitState): void {
    const oldState = this.state;
    this.state = newState;

    // Reset counters on state transitions
    if (newState === "closed") {
      this.failures = 0;
      this.halfOpenSuccesses = 0;
    } else if (newState === "half-open") {
      this.halfOpenSuccesses = 0;
    }

    configLogger.info(
      {
        provider: this.providerName,
        oldState,
        newState,
        failures: this.failures,
        resetTimeoutMs: this.config.resetTimeoutMs,
      },
      `Circuit breaker state transition: ${oldState} → ${newState} for ${this.providerName}`
    );

    // Notify persistence layer of state change
    this.notifyChange();
  }

  /**
   * Notify persistence layer of state change
   */
  private notifyChange(): void {
    if (this.onChange) {
      try {
        this.onChange(this.providerName, this.getStats());
      } catch (error) {
        configLogger.error(
          { error: error instanceof Error ? error.message : String(error) },
          `Failed to notify change for ${this.providerName}`
        );
      }
    }
  }
}

/**
 * Manager for multiple circuit breakers (one per provider)
 */
export class CircuitBreakerManager {
  private breakers = new Map<string, CircuitBreaker>();
  private onChange?: CircuitBreakerOnChange;

  constructor(private readonly config: CircuitBreakerConfig = DEFAULT_CIRCUIT_BREAKER_CONFIG) {}

  /**
   * Set callback for all circuit breaker state changes (used for persistence)
   */
  setOnChange(callback: CircuitBreakerOnChange): void {
    this.onChange = callback;
    // Apply to existing breakers
    for (const breaker of this.breakers.values()) {
      breaker.setOnChange(callback);
    }
  }

  /**
   * Restore state for multiple providers from persisted data
   */
  restoreStates(states: Record<string, PersistedCircuitState>): void {
    for (const [providerName, persisted] of Object.entries(states)) {
      const breaker = this.getBreaker(providerName);
      breaker.restoreState(persisted);
    }
    configLogger.info(
      { providerCount: Object.keys(states).length },
      "Circuit breaker states restored from persistence"
    );
  }

  /**
   * Get or create a circuit breaker for a provider
   */
  getBreaker(providerName: string): CircuitBreaker {
    let breaker = this.breakers.get(providerName);
    if (!breaker) {
      breaker = new CircuitBreaker(providerName, this.config);
      // Apply onChange callback if set
      if (this.onChange) {
        breaker.setOnChange(this.onChange);
      }
      this.breakers.set(providerName, breaker);
    }
    return breaker;
  }

  /**
   * Get all healthy providers (circuit closed or half-open)
   */
  getHealthyProviders(): string[] {
    const healthy: string[] = [];
    for (const [name, breaker] of this.breakers) {
      if (breaker.canRequest()) {
        healthy.push(name);
      }
    }
    return healthy;
  }

  /**
   * Get stats for all circuit breakers
   */
  getAllStats(): Record<string, CircuitBreakerStats> {
    const stats: Record<string, CircuitBreakerStats> = {};
    for (const [name, breaker] of this.breakers) {
      stats[name] = breaker.getStats();
    }
    return stats;
  }

  /**
   * Reset all circuit breakers
   */
  resetAll(): void {
    for (const breaker of this.breakers.values()) {
      breaker.reset();
    }
    configLogger.info({}, "All circuit breakers have been reset");
  }
}
