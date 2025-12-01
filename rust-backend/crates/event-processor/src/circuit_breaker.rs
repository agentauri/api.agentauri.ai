//! Circuit Breaker Pattern for Trigger Reliability
//!
//! Implements a state machine-based circuit breaker to prevent cascade failures
//! when triggers repeatedly fail. Automatically disables failing triggers and
//! re-enables them after a recovery period.
//!
//! # State Machine
//!
//! ```text
//! CLOSED (normal operation)
//!   ↓ (10 consecutive failures)
//! OPEN (trigger disabled, rejects all events)
//!   ↓ (after 1 hour)
//! HALF-OPEN (test mode, allows 1 event through)
//!   ↓ (success) → CLOSED
//!   ↓ (failure) → OPEN
//! ```
//!
//! # States
//!
//! - **Closed**: Normal operation, all events processed
//! - **Open**: Trigger disabled, events rejected immediately (fail-fast)
//! - **Half-Open**: Recovery test, allows 1 event to verify health
//!
//! # Configuration
//!
//! Per-trigger configuration stored in `triggers.circuit_breaker_config`:
//! - `failure_threshold`: Number of consecutive failures before opening (default: 10)
//! - `recovery_timeout_seconds`: Time to wait before attempting recovery (default: 3600)
//! - `half_open_max_calls`: Maximum calls allowed in half-open state (default: 1)
//!
//! # Persistence
//!
//! Circuit breaker state is persisted to PostgreSQL in `triggers.circuit_breaker_state`
//! for recovery after service restarts.
//!
//! # Thread Safety
//!
//! Uses `Arc<RwLock<CircuitBreakerState>>` for concurrent access from multiple async tasks.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation - all events processed
    Closed,
    /// Trigger disabled - events rejected immediately
    Open,
    /// Recovery test mode - allows limited events through
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// Circuit breaker configuration (per-trigger)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting recovery (seconds)
    pub recovery_timeout_seconds: u64,
    /// Maximum calls allowed in half-open state
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout_seconds: 3600, // 1 hour
            half_open_max_calls: 1,
        }
    }
}

/// Circuit breaker state (per-trigger)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    /// Current circuit state
    pub state: CircuitState,
    /// Consecutive failure count
    pub failure_count: u32,
    /// Timestamp of last failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_time: Option<DateTime<Utc>>,
    /// Timestamp when circuit was opened
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened_at: Option<DateTime<Utc>>,
    /// Number of calls made in half-open state
    pub half_open_calls: u32,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_time: None,
            opened_at: None,
            half_open_calls: 0,
        }
    }
}

/// Circuit Breaker for trigger reliability
///
/// Prevents cascade failures by tracking execution failures and automatically
/// disabling triggers that fail repeatedly.
pub struct CircuitBreaker {
    /// Trigger ID this circuit breaker protects
    trigger_id: String,
    /// Circuit breaker configuration
    config: CircuitBreakerConfig,
    /// Current circuit breaker state (thread-safe)
    state: Arc<RwLock<CircuitBreakerState>>,
    /// Database connection pool for persistence
    db_pool: PgPool,
}

impl CircuitBreaker {
    /// Create new circuit breaker from database
    ///
    /// Loads configuration and state from the triggers table.
    ///
    /// # Arguments
    ///
    /// * `trigger_id` - ID of the trigger to protect
    /// * `db_pool` - Database connection pool
    ///
    /// # Returns
    ///
    /// Circuit breaker instance with loaded configuration and state
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Database query fails
    /// - Trigger not found
    /// - Invalid configuration/state JSON
    pub async fn new(trigger_id: String, db_pool: PgPool) -> Result<Self> {
        // Load configuration and state from database
        let record = sqlx::query!(
            r#"
            SELECT
                circuit_breaker_config,
                circuit_breaker_state
            FROM triggers
            WHERE id = $1
            "#,
            trigger_id
        )
        .fetch_optional(&db_pool)
        .await
        .context("Failed to fetch trigger from database")?
        .ok_or_else(|| anyhow::anyhow!("Trigger not found: {}", trigger_id))?;

        // Parse configuration (use default if not set)
        let config: CircuitBreakerConfig = record
            .circuit_breaker_config
            .as_ref()
            .map(|json| serde_json::from_value(json.clone()))
            .transpose()
            .context("Failed to parse circuit_breaker_config")?
            .unwrap_or_default();

        // Parse state (use default if not set)
        let state: CircuitBreakerState = record
            .circuit_breaker_state
            .as_ref()
            .map(|json| serde_json::from_value(json.clone()))
            .transpose()
            .context("Failed to parse circuit_breaker_state")?
            .unwrap_or_default();

        debug!(
            trigger_id = %trigger_id,
            state = %state.state,
            failure_count = state.failure_count,
            "Loaded circuit breaker from database"
        );

        Ok(Self {
            trigger_id,
            config,
            state: Arc::new(RwLock::new(state)),
            db_pool,
        })
    }

    /// Check if request is allowed (fail-fast if Open)
    ///
    /// # Returns
    ///
    /// - `true` if event should be processed
    /// - `false` if event should be rejected (circuit open)
    ///
    /// # State Transitions
    ///
    /// - **Closed**: Always allow
    /// - **Open**: Check if recovery timeout passed → Half-Open, otherwise deny
    /// - **Half-Open**: Allow if half_open_calls < half_open_max_calls
    pub async fn call_allowed(&self) -> Result<bool> {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::Closed => {
                // Normal operation - allow all calls
                Ok(true)
            }
            CircuitState::Open => {
                // Check if recovery timeout has passed
                if self.should_attempt_reset(&state) {
                    // Transition to Half-Open
                    info!(
                        trigger_id = %self.trigger_id,
                        "Circuit breaker transitioning to Half-Open (recovery timeout passed)"
                    );

                    state.state = CircuitState::HalfOpen;
                    state.half_open_calls = 0;

                    // Persist state transition
                    drop(state); // Release lock before async operation
                    self.persist_state().await?;

                    Ok(true)
                } else {
                    // Still in recovery period - reject call
                    debug!(
                        trigger_id = %self.trigger_id,
                        "Circuit breaker OPEN - rejecting call"
                    );
                    Ok(false)
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited calls for testing
                if state.half_open_calls < self.config.half_open_max_calls {
                    state.half_open_calls += 1;
                    debug!(
                        trigger_id = %self.trigger_id,
                        half_open_calls = state.half_open_calls,
                        max_calls = self.config.half_open_max_calls,
                        "Circuit breaker Half-Open - allowing test call"
                    );
                    Ok(true)
                } else {
                    debug!(
                        trigger_id = %self.trigger_id,
                        "Circuit breaker Half-Open - max calls reached, rejecting"
                    );
                    Ok(false)
                }
            }
        }
    }

    /// Record successful execution
    ///
    /// # State Transitions
    ///
    /// - **Closed**: Reset failure_count to 0
    /// - **Half-Open**: Transition to Closed, reset counters
    /// - **Open**: Should not happen (calls are blocked)
    pub async fn record_success(&self) -> Result<()> {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::Closed => {
                // Reset failure count on success
                if state.failure_count > 0 {
                    debug!(
                        trigger_id = %self.trigger_id,
                        previous_failures = state.failure_count,
                        "Resetting failure count after success"
                    );
                    state.failure_count = 0;
                    state.last_failure_time = None;

                    // Persist state
                    drop(state);
                    self.persist_state().await?;
                }
            }
            CircuitState::HalfOpen => {
                // Success in half-open means recovery is successful
                info!(
                    trigger_id = %self.trigger_id,
                    "Circuit breaker transitioning to Closed (recovery successful)"
                );

                state.state = CircuitState::Closed;
                state.failure_count = 0;
                state.last_failure_time = None;
                state.opened_at = None;
                state.half_open_calls = 0;

                // Persist state transition
                drop(state);
                self.persist_state().await?;
            }
            CircuitState::Open => {
                // This should not happen - calls should be blocked
                warn!(
                    trigger_id = %self.trigger_id,
                    "Received success in Open state (unexpected)"
                );
            }
        }

        Ok(())
    }

    /// Record failed execution
    ///
    /// # State Transitions
    ///
    /// - **Closed**: Increment failure_count, if >= threshold → Open
    /// - **Half-Open**: Transition back to Open
    /// - **Open**: No-op (already open)
    pub async fn record_failure(&self) -> Result<()> {
        let mut state = self.state.write().await;
        let now = Utc::now();

        match state.state {
            CircuitState::Closed => {
                state.failure_count += 1;
                state.last_failure_time = Some(now);

                debug!(
                    trigger_id = %self.trigger_id,
                    failure_count = state.failure_count,
                    threshold = self.config.failure_threshold,
                    "Recorded failure in Closed state"
                );

                // Check if threshold exceeded
                if state.failure_count >= self.config.failure_threshold {
                    // Transition to Open
                    warn!(
                        trigger_id = %self.trigger_id,
                        failure_count = state.failure_count,
                        threshold = self.config.failure_threshold,
                        "Circuit breaker transitioning to Open (failure threshold exceeded)"
                    );

                    state.state = CircuitState::Open;
                    state.opened_at = Some(now);

                    // Persist state transition
                    drop(state);
                    self.persist_state().await?;
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open means recovery failed
                warn!(
                    trigger_id = %self.trigger_id,
                    "Circuit breaker transitioning to Open (recovery failed)"
                );

                state.state = CircuitState::Open;
                state.opened_at = Some(now);
                state.last_failure_time = Some(now);
                state.half_open_calls = 0;

                // Persist state transition
                drop(state);
                self.persist_state().await?;
            }
            CircuitState::Open => {
                // Already open - no action needed
                debug!(
                    trigger_id = %self.trigger_id,
                    "Received failure in Open state (already open)"
                );
            }
        }

        Ok(())
    }

    /// Get current state (for metrics/observability)
    pub async fn get_state(&self) -> CircuitState {
        let state = self.state.read().await;
        state.state
    }

    /// Persist state to database
    ///
    /// Updates `triggers.circuit_breaker_state` column.
    ///
    /// # Error Handling
    ///
    /// Database errors are logged but not propagated. This provides graceful
    /// degradation - the circuit breaker continues working with in-memory state.
    async fn persist_state(&self) -> Result<()> {
        let state = self.state.read().await;
        let state_json = serde_json::to_value(&*state).context("Failed to serialize state")?;

        match sqlx::query!(
            r#"
            UPDATE triggers
            SET circuit_breaker_state = $1
            WHERE id = $2
            "#,
            state_json,
            self.trigger_id
        )
        .execute(&self.db_pool)
        .await
        {
            Ok(_) => {
                debug!(
                    trigger_id = %self.trigger_id,
                    state = %state.state,
                    "Persisted circuit breaker state to database"
                );
                Ok(())
            }
            Err(e) => {
                // Log error but don't propagate - graceful degradation
                error!(
                    trigger_id = %self.trigger_id,
                    error = %e,
                    "Failed to persist circuit breaker state (using in-memory state)"
                );
                // Return Ok to continue processing with in-memory state
                Ok(())
            }
        }
    }

    /// Check if should transition from Open to Half-Open
    ///
    /// Returns true if recovery timeout has passed since circuit was opened.
    fn should_attempt_reset(&self, state: &CircuitBreakerState) -> bool {
        if let Some(opened_at) = state.opened_at {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(opened_at);
            let timeout = chrono::Duration::seconds(self.config.recovery_timeout_seconds as i64);

            elapsed >= timeout
        } else {
            // No opened_at timestamp - should not happen in Open state
            warn!(
                trigger_id = %self.trigger_id,
                "Open state missing opened_at timestamp"
            );
            true // Allow reset attempt
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.recovery_timeout_seconds, 3600);
        assert_eq!(config.half_open_max_calls, 1);
    }

    #[test]
    fn test_default_state() {
        let state = CircuitBreakerState::default();
        assert_eq!(state.state, CircuitState::Closed);
        assert_eq!(state.failure_count, 0);
        assert!(state.last_failure_time.is_none());
        assert!(state.opened_at.is_none());
        assert_eq!(state.half_open_calls, 0);
    }

    #[test]
    fn test_circuit_state_display() {
        assert_eq!(CircuitState::Closed.to_string(), "Closed");
        assert_eq!(CircuitState::Open.to_string(), "Open");
        assert_eq!(CircuitState::HalfOpen.to_string(), "HalfOpen");
    }

    #[test]
    fn test_config_serialization() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            recovery_timeout_seconds: 1800,
            half_open_max_calls: 2,
        };

        let json = serde_json::to_value(&config).unwrap();
        let deserialized: CircuitBreakerConfig = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.failure_threshold, 5);
        assert_eq!(deserialized.recovery_timeout_seconds, 1800);
        assert_eq!(deserialized.half_open_max_calls, 2);
    }

    #[test]
    fn test_state_serialization() {
        let state = CircuitBreakerState {
            state: CircuitState::Open,
            failure_count: 10,
            last_failure_time: Some(Utc::now()),
            opened_at: Some(Utc::now()),
            half_open_calls: 0,
        };

        let json = serde_json::to_value(&state).unwrap();
        let deserialized: CircuitBreakerState = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.state, CircuitState::Open);
        assert_eq!(deserialized.failure_count, 10);
        assert!(deserialized.last_failure_time.is_some());
        assert!(deserialized.opened_at.is_some());
    }
}
