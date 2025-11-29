//! Rate Counter evaluator
//!
//! Counts events in a sliding time window to detect spam, abuse, or anomalous activity.
//! Useful for triggering alerts when event frequency exceeds expected thresholds.
//!
//! # Algorithm
//!
//! Maintains a list of recent event timestamps and counts events within the time window.
//! Old timestamps outside the window are automatically pruned.
//!
//! # Example
//!
//! ```json
//! {
//!   "condition_type": "rate_limit",
//!   "field": "event_count",
//!   "operator": ">",
//!   "value": "10",
//!   "config": {
//!     "time_window": "1h",
//!     "reset_on_trigger": false
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use shared::models::{Event, TriggerCondition};

/// Maximum number of timestamps to store (防止内存爆炸)
const MAX_TIMESTAMPS: usize = 10_000;

/// Rate counter state stored in trigger_state table
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateCounterState {
    /// Start of the current window
    pub window_start: DateTime<Utc>,
    /// Number of events in current window
    pub count: u32,
    /// Recent event timestamps (Unix seconds)
    pub recent_timestamps: Vec<i64>,
}

/// Rate counter evaluator for event frequency conditions
#[derive(Debug)]
pub struct RateCounterEvaluator {
    time_window: Duration,
    reset_on_trigger: bool,
}

impl RateCounterEvaluator {
    /// Create evaluator from condition config JSONB
    ///
    /// # Config Format
    ///
    /// ```json
    /// {
    ///   "time_window": "1h",
    ///   "reset_on_trigger": false
    /// }
    /// ```
    ///
    /// Time window formats:
    /// - "10s" - 10 seconds
    /// - "5m" - 5 minutes
    /// - "1h" - 1 hour
    /// - "7d" - 7 days
    ///
    /// # Errors
    ///
    /// Returns error if time_window is missing, invalid, or zero
    pub fn from_config(config: &serde_json::Value) -> Result<Self> {
        let time_window_str = config
            .get("time_window")
            .and_then(|v| v.as_str())
            .context("Missing or invalid time_window in config")?;

        let time_window = parse_duration(time_window_str)
            .with_context(|| format!("Invalid time_window format: {}", time_window_str))?;

        if time_window.num_seconds() == 0 {
            anyhow::bail!("time_window must be greater than 0");
        }

        let reset_on_trigger = config
            .get("reset_on_trigger")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            time_window,
            reset_on_trigger,
        })
    }

    /// Evaluate rate counter condition against an event
    ///
    /// # Arguments
    ///
    /// * `event` - Event to evaluate
    /// * `condition` - Condition with operator and threshold value
    /// * `current_state` - Current rate counter state (None for first event)
    ///
    /// # Returns
    ///
    /// Tuple of (matches, new_state):
    /// - matches: true if condition is satisfied
    /// - new_state: updated rate counter state for persistence
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Threshold value cannot be parsed
    /// - Operator is invalid
    pub fn evaluate(
        &self,
        event: &Event,
        condition: &TriggerCondition,
        current_state: Option<RateCounterState>,
    ) -> Result<(bool, RateCounterState)> {
        let now = Utc::now();
        let event_timestamp = event.timestamp;

        tracing::trace!(
            event_timestamp = event_timestamp,
            current_count = ?current_state.as_ref().map(|s| s.count),
            "Evaluating rate counter condition"
        );

        // Initialize or load state
        let mut state = current_state.unwrap_or_else(|| RateCounterState {
            window_start: now - self.time_window,
            count: 0,
            recent_timestamps: Vec::new(),
        });

        // Remove timestamps outside the window
        let cutoff = (now - self.time_window).timestamp();
        let original_len = state.recent_timestamps.len();
        state.recent_timestamps.retain(|&ts| ts >= cutoff);
        let removed = original_len - state.recent_timestamps.len();

        if removed > 0 {
            tracing::trace!(removed = removed, "Pruned old timestamps");
        }

        // Add current event timestamp
        state.recent_timestamps.push(event_timestamp);

        // Enforce maximum timestamp limit to prevent memory explosion
        if state.recent_timestamps.len() > MAX_TIMESTAMPS {
            tracing::warn!(
                count = state.recent_timestamps.len(),
                max = MAX_TIMESTAMPS,
                "Timestamp list exceeds maximum, truncating oldest"
            );
            // Keep only the most recent MAX_TIMESTAMPS
            state.recent_timestamps = state
                .recent_timestamps
                .iter()
                .rev()
                .take(MAX_TIMESTAMPS)
                .rev()
                .copied()
                .collect();
        }

        state.count = state.recent_timestamps.len() as u32;
        state.window_start = now - self.time_window;

        // Extract threshold and operator
        let threshold = condition
            .value
            .parse::<u32>()
            .with_context(|| format!("Invalid threshold value: {}", condition.value))?;

        let operator = condition.operator.as_str();

        // Evaluate condition
        let matches = match operator {
            ">" => state.count > threshold,
            ">=" => state.count >= threshold,
            "<" => state.count < threshold,
            "<=" => state.count <= threshold,
            "=" | "==" => state.count == threshold,
            "!=" | "<>" => state.count != threshold,
            _ => anyhow::bail!("Invalid operator: {}", operator),
        };

        tracing::debug!(
            count = state.count,
            threshold = threshold,
            operator = operator,
            matches = matches,
            timestamps_len = state.recent_timestamps.len(),
            "Rate counter evaluation complete"
        );

        // Reset counter if configured and triggered
        if matches && self.reset_on_trigger {
            tracing::info!("Condition triggered, resetting rate counter");
            state.count = 0;
            state.recent_timestamps.clear();
        }

        Ok((matches, state))
    }
}

/// Parse duration string into chrono::Duration
///
/// Supported formats:
/// - "10s" - 10 seconds
/// - "5m" - 5 minutes
/// - "1h" - 1 hour
/// - "7d" - 7 days
fn parse_duration(s: &str) -> Result<Duration> {
    if s.is_empty() {
        anyhow::bail!("Duration string is empty");
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: i64 = num_str
        .parse()
        .with_context(|| format!("Invalid number in duration: {}", num_str))?;

    if num <= 0 {
        anyhow::bail!("Duration must be positive");
    }

    match unit {
        "s" => Ok(Duration::seconds(num)),
        "m" => Ok(Duration::minutes(num)),
        "h" => Ok(Duration::hours(num)),
        "d" => Ok(Duration::days(num)),
        _ => anyhow::bail!("Invalid time unit: {} (expected s, m, h, d)", unit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test event with customizable timestamp
    fn create_test_event(timestamp: i64) -> Event {
        Event {
            id: format!("test-event-{}", timestamp),
            chain_id: 84532,
            block_number: 1000,
            block_hash: "0xabc".to_string(),
            transaction_hash: "0xdef".to_string(),
            log_index: 0,
            registry: "reputation".to_string(),
            event_type: "NewFeedback".to_string(),
            agent_id: Some(42),
            timestamp,
            owner: None,
            token_uri: None,
            metadata_key: None,
            metadata_value: None,
            client_address: Some("0x123".to_string()),
            feedback_index: Some(0),
            score: Some(85),
            tag1: Some("trade".to_string()),
            tag2: Some("reliable".to_string()),
            file_uri: None,
            file_hash: None,
            validator_address: None,
            request_hash: None,
            response: None,
            response_uri: None,
            response_hash: None,
            tag: None,
            created_at: Utc::now(),
        }
    }

    /// Create a test condition
    fn create_test_condition(operator: &str, value: &str) -> TriggerCondition {
        TriggerCondition {
            id: 1,
            trigger_id: "test-trigger".to_string(),
            condition_type: "rate_limit".to_string(),
            field: "event_count".to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
            config: Some(serde_json::json!({
                "time_window": "1h",
                "reset_on_trigger": false
            })),
            created_at: Utc::now(),
        }
    }

    /// Create a state with N events in the last hour
    fn create_state_with_events(count: usize) -> RateCounterState {
        let now = Utc::now();
        let timestamps: Vec<i64> = (0..count)
            .map(|i| (now - Duration::minutes(i as i64)).timestamp())
            .collect();

        RateCounterState {
            window_start: now - Duration::hours(1),
            count: count as u32,
            recent_timestamps: timestamps,
        }
    }

    // ========================================================================
    // parse_duration tests
    // ========================================================================

    #[test]
    fn test_parse_duration_seconds() {
        let duration = parse_duration("30s").unwrap();
        assert_eq!(duration.num_seconds(), 30);
    }

    #[test]
    fn test_parse_duration_minutes() {
        let duration = parse_duration("15m").unwrap();
        assert_eq!(duration.num_minutes(), 15);
    }

    #[test]
    fn test_parse_duration_hours() {
        let duration = parse_duration("2h").unwrap();
        assert_eq!(duration.num_hours(), 2);
    }

    #[test]
    fn test_parse_duration_days() {
        let duration = parse_duration("7d").unwrap();
        assert_eq!(duration.num_days(), 7);
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        let result = parse_duration("10x");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid time unit"));
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        let result = parse_duration("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duration_empty() {
        let result = parse_duration("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duration_zero() {
        let result = parse_duration("0s");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("positive"));
    }

    #[test]
    fn test_parse_duration_negative() {
        let result = parse_duration("-5m");
        assert!(result.is_err());
    }

    // ========================================================================
    // from_config tests
    // ========================================================================

    #[test]
    fn test_from_config_valid() {
        let config = serde_json::json!({
            "time_window": "1h",
            "reset_on_trigger": true
        });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();
        assert_eq!(evaluator.time_window.num_hours(), 1);
        assert!(evaluator.reset_on_trigger);
    }

    #[test]
    fn test_from_config_default_reset() {
        let config = serde_json::json!({ "time_window": "30m" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();
        assert_eq!(evaluator.time_window.num_minutes(), 30);
        assert!(!evaluator.reset_on_trigger); // default false
    }

    #[test]
    fn test_from_config_missing_time_window() {
        let config = serde_json::json!({});
        let result = RateCounterEvaluator::from_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("time_window"));
    }

    #[test]
    fn test_from_config_zero_time_window() {
        let config = serde_json::json!({ "time_window": "0s" });
        let result = RateCounterEvaluator::from_config(&config);
        assert!(result.is_err());
    }

    // ========================================================================
    // Basic evaluation tests
    // ========================================================================

    #[test]
    fn test_rate_counter_first_event() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "0");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();

        assert_eq!(state.count, 1);
        assert_eq!(state.recent_timestamps.len(), 1);
        assert!(matches); // 1 > 0
    }

    #[test]
    fn test_rate_counter_below_threshold() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "10");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();

        assert_eq!(state.count, 1);
        assert!(!matches); // 1 is NOT > 10
    }

    #[test]
    fn test_rate_counter_exceeds_threshold() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        // Create state with 11 events
        let state = create_state_with_events(11);

        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "10");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert_eq!(new_state.count, 12); // 11 + 1
        assert!(matches); // 12 > 10
    }

    // ========================================================================
    // Sliding window tests
    // ========================================================================

    #[test]
    fn test_rate_counter_prunes_old_timestamps() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let now = Utc::now();
        // Create state with timestamps from 2 hours ago (outside window)
        let old_timestamps = vec![
            (now - Duration::hours(2)).timestamp(),
            (now - Duration::hours(3)).timestamp(),
        ];

        let state = RateCounterState {
            window_start: now - Duration::hours(1),
            count: 2,
            recent_timestamps: old_timestamps,
        };

        let event = create_test_event(now.timestamp());
        let condition = create_test_condition("=", "1");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        // Old timestamps should be pruned, only new event remains
        assert_eq!(new_state.count, 1);
        assert_eq!(new_state.recent_timestamps.len(), 1);
        assert!(matches); // 1 == 1
    }

    #[test]
    fn test_rate_counter_keeps_recent_timestamps() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let now = Utc::now();
        // Create state with timestamps from 30 minutes ago (inside window)
        let recent_timestamps = vec![
            (now - Duration::minutes(30)).timestamp(),
            (now - Duration::minutes(20)).timestamp(),
        ];

        let state = RateCounterState {
            window_start: now - Duration::hours(1),
            count: 2,
            recent_timestamps: recent_timestamps.clone(),
        };

        let event = create_test_event(now.timestamp());
        let condition = create_test_condition("=", "3");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        // Recent timestamps should be kept + new event
        assert_eq!(new_state.count, 3);
        assert_eq!(new_state.recent_timestamps.len(), 3);
        assert!(matches); // 3 == 3
    }

    #[test]
    fn test_rate_counter_mixed_timestamps() {
        let config = serde_json::json!({ "time_window": "30m" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let now = Utc::now();
        // Mix of old and recent timestamps
        let timestamps = vec![
            (now - Duration::hours(2)).timestamp(),    // too old
            (now - Duration::minutes(45)).timestamp(), // too old
            (now - Duration::minutes(20)).timestamp(), // keep
            (now - Duration::minutes(10)).timestamp(), // keep
        ];

        let state = RateCounterState {
            window_start: now - Duration::minutes(30),
            count: 4,
            recent_timestamps: timestamps,
        };

        let event = create_test_event(now.timestamp());
        let condition = create_test_condition("=", "3");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        // Should keep 2 recent + 1 new = 3
        assert_eq!(new_state.count, 3);
        assert!(matches); // 3 == 3
    }

    // ========================================================================
    // Operator tests
    // ========================================================================

    #[test]
    fn test_rate_counter_operator_greater_than() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(5);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "5");

        let (matches, _) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 6 > 5
    }

    #[test]
    fn test_rate_counter_operator_less_than() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(3);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition("<", "10");

        let (matches, _) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 4 < 10
    }

    #[test]
    fn test_rate_counter_operator_greater_or_equal() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(4);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">=", "5");

        let (matches, _) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 5 >= 5
    }

    #[test]
    fn test_rate_counter_operator_less_or_equal() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(4);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition("<=", "5");

        let (matches, _) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 5 <= 5
    }

    #[test]
    fn test_rate_counter_operator_equals() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition("=", "1");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 1 == 1
    }

    #[test]
    fn test_rate_counter_operator_not_equals() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(5);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition("!=", "10");

        let (matches, _) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 6 != 10
    }

    #[test]
    fn test_rate_counter_operator_invalid() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let event = create_test_event(Utc::now().timestamp());
        let mut condition = create_test_condition("~", "5");
        condition.operator = "~".to_string();

        let result = evaluator.evaluate(&event, &condition, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid operator"));
    }

    // ========================================================================
    // Reset on trigger tests
    // ========================================================================

    #[test]
    fn test_rate_counter_reset_on_trigger() {
        let config = serde_json::json!({
            "time_window": "1h",
            "reset_on_trigger": true
        });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(10);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "10");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 11 > 10 triggers
        assert_eq!(new_state.count, 0); // RESET after trigger
        assert!(new_state.recent_timestamps.is_empty());
    }

    #[test]
    fn test_rate_counter_no_reset_when_not_triggered() {
        let config = serde_json::json!({
            "time_window": "1h",
            "reset_on_trigger": true
        });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(5);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "10");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(!matches); // 6 is NOT > 10, no trigger
        assert_eq!(new_state.count, 6); // NOT reset
        assert_eq!(new_state.recent_timestamps.len(), 6);
    }

    #[test]
    fn test_rate_counter_no_reset_when_disabled() {
        let config = serde_json::json!({
            "time_window": "1h",
            "reset_on_trigger": false
        });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let state = create_state_with_events(10);
        let event = create_test_event(Utc::now().timestamp());
        let condition = create_test_condition(">", "10");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        assert!(matches); // 11 > 10 triggers
        assert_eq!(new_state.count, 11); // NOT reset (disabled)
        assert_eq!(new_state.recent_timestamps.len(), 11);
    }

    // ========================================================================
    // Edge case tests
    // ========================================================================

    #[test]
    fn test_rate_counter_max_timestamps_limit() {
        let config = serde_json::json!({ "time_window": "100d" }); // Very long window to keep all timestamps
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        // Create state with MAX_TIMESTAMPS + 1000 events (over limit)
        // All within the 100d window (spaced 1s apart)
        let now = Utc::now();
        let timestamps: Vec<i64> = (0..(MAX_TIMESTAMPS + 1000))
            .map(|i| (now - Duration::seconds(i as i64)).timestamp()) // 1s apart
            .collect();

        let state = RateCounterState {
            window_start: now - Duration::days(100),
            count: (MAX_TIMESTAMPS + 1000) as u32,
            recent_timestamps: timestamps,
        };

        let event = create_test_event(now.timestamp());
        let condition = create_test_condition(">", "0");

        let (_, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        // Should be truncated to MAX_TIMESTAMPS
        assert_eq!(new_state.recent_timestamps.len(), MAX_TIMESTAMPS);
        assert_eq!(new_state.count, MAX_TIMESTAMPS as u32);
    }

    #[test]
    fn test_rate_counter_invalid_threshold() {
        let config = serde_json::json!({ "time_window": "1h" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let event = create_test_event(Utc::now().timestamp());
        let mut condition = create_test_condition(">", "not_a_number");
        condition.value = "not_a_number".to_string();

        let result = evaluator.evaluate(&event, &condition, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid threshold"));
    }

    #[test]
    fn test_rate_counter_state_serialization() {
        let state = RateCounterState {
            window_start: Utc::now(),
            count: 5,
            recent_timestamps: vec![1234567890, 1234567900],
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: RateCounterState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.count, deserialized.count);
        assert_eq!(state.recent_timestamps, deserialized.recent_timestamps);
    }

    // ========================================================================
    // Different time window tests
    // ========================================================================

    #[test]
    fn test_rate_counter_short_window_seconds() {
        let config = serde_json::json!({ "time_window": "30s" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let now = Utc::now();
        // Events from 1 minute ago (outside 30s window)
        let old_timestamps = vec![(now - Duration::seconds(60)).timestamp()];

        let state = RateCounterState {
            window_start: now - Duration::seconds(30),
            count: 1,
            recent_timestamps: old_timestamps,
        };

        let event = create_test_event(now.timestamp());
        let condition = create_test_condition("=", "1");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        // Old event pruned, only new event
        assert_eq!(new_state.count, 1);
        assert!(matches);
    }

    #[test]
    fn test_rate_counter_long_window_days() {
        let config = serde_json::json!({ "time_window": "7d" });
        let evaluator = RateCounterEvaluator::from_config(&config).unwrap();

        let now = Utc::now();
        // Events from 3 days ago (within 7d window)
        let timestamps = vec![(now - Duration::days(3)).timestamp()];

        let state = RateCounterState {
            window_start: now - Duration::days(7),
            count: 1,
            recent_timestamps: timestamps,
        };

        let event = create_test_event(now.timestamp());
        let condition = create_test_condition("=", "2");

        let (matches, new_state) = evaluator.evaluate(&event, &condition, Some(state)).unwrap();

        // Old event kept, plus new event = 2
        assert_eq!(new_state.count, 2);
        assert!(matches);
    }
}
