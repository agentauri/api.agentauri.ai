-- Migration: Add Circuit Breaker Columns to Triggers Table
-- Description: Adds circuit breaker configuration and state columns for reliability pattern
-- Created: 2025-11-30
--
-- Circuit Breaker Pattern:
--   - Tracks failure counts per trigger
--   - Auto-disables triggers after N consecutive failures
--   - Implements state machine: Closed → Open → Half-Open
--   - Auto-recovery after configurable timeout
--
-- State Machine:
--   CLOSED (normal)
--     ↓ (N consecutive failures)
--   OPEN (disabled)
--     ↓ (after timeout)
--   HALF-OPEN (testing)
--     ↓ (success) → CLOSED
--     ↓ (failure) → OPEN

-- Add circuit breaker configuration column
-- Contains: failure_threshold, recovery_timeout_seconds, half_open_max_calls
ALTER TABLE triggers
ADD COLUMN circuit_breaker_config JSONB DEFAULT '{
  "failure_threshold": 10,
  "recovery_timeout_seconds": 3600,
  "half_open_max_calls": 1
}'::jsonb;

-- Add circuit breaker state column
-- Contains: state, failure_count, last_failure_time, opened_at, half_open_calls
ALTER TABLE triggers
ADD COLUMN circuit_breaker_state JSONB DEFAULT '{
  "state": "Closed",
  "failure_count": 0,
  "half_open_calls": 0
}'::jsonb;

-- Index for querying open circuit breakers (for monitoring/alerting)
CREATE INDEX idx_triggers_circuit_breaker_state
ON triggers ((circuit_breaker_state->>'state'))
WHERE (circuit_breaker_state->>'state') IN ('Open', 'HalfOpen');

-- Comment on columns
COMMENT ON COLUMN triggers.circuit_breaker_config IS 'Circuit breaker configuration: failure_threshold, recovery_timeout_seconds, half_open_max_calls';
COMMENT ON COLUMN triggers.circuit_breaker_state IS 'Circuit breaker state: state (Closed/Open/HalfOpen), failure_count, last_failure_time, opened_at, half_open_calls';
