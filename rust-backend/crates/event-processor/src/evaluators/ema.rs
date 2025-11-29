//! Exponential Moving Average (EMA) evaluator
//!
//! Smooths score trends to detect gradual changes in agent reputation.
//! Useful for filtering out noise and focusing on sustained trends.
//!
//! # Algorithm
//!
//! EMA uses a smoothing factor (alpha) to give more weight to recent values:
//! - alpha = 2 / (window_size + 1)
//! - EMA_new = alpha * score + (1 - alpha) * EMA_old
//!
//! # Example
//!
//! ```json
//! {
//!   "condition_type": "ema_threshold",
//!   "field": "score",
//!   "operator": "<",
//!   "value": "70",
//!   "config": {
//!     "window_size": 10
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::models::{Event, TriggerCondition};

/// EMA state stored in trigger_state table
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EmaState {
    /// Current exponential moving average value
    pub ema: f64,
    /// Number of events processed
    pub count: usize,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

/// EMA evaluator for score-based conditions
#[derive(Debug)]
pub struct EmaEvaluator {
    window_size: usize,
    alpha: f64, // smoothing factor (0.0 to 1.0)
}

impl EmaEvaluator {
    /// Create a new EMA evaluator with the specified window size
    ///
    /// # Arguments
    ///
    /// * `window_size` - Number of data points to consider (higher = smoother)
    ///
    /// # Examples
    ///
    /// ```
    /// let evaluator = EmaEvaluator::new(10); // 10-period EMA
    /// ```
    pub fn new(window_size: usize) -> Self {
        // Alpha calculation: 2 / (window_size + 1)
        // For window_size = 10: alpha ≈ 0.1818
        let alpha = 2.0 / (window_size as f64 + 1.0);
        Self { window_size, alpha }
    }

    /// Create evaluator from condition config JSONB
    ///
    /// # Config Format
    ///
    /// ```json
    /// {
    ///   "window_size": 10
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if window_size is 0 or missing
    pub fn from_config(config: &serde_json::Value) -> Result<Self> {
        let window_size = config
            .get("window_size")
            .and_then(|v| v.as_u64())
            .context("Missing or invalid window_size in config")?
            as usize;

        if window_size == 0 {
            anyhow::bail!("window_size must be greater than 0");
        }

        Ok(Self::new(window_size))
    }

    /// Evaluate EMA condition against an event
    ///
    /// # Arguments
    ///
    /// * `event` - Event to evaluate (must have score field)
    /// * `condition` - Condition with operator and threshold value
    /// * `current_state` - Current EMA state (None for first event)
    ///
    /// # Returns
    ///
    /// Tuple of (matches, new_state):
    /// - matches: true if condition is satisfied
    /// - new_state: updated EMA state for persistence
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Event has no score field
    /// - Threshold value cannot be parsed
    /// - Operator is invalid
    pub fn evaluate(
        &self,
        event: &Event,
        condition: &TriggerCondition,
        current_state: Option<EmaState>,
    ) -> Result<(bool, EmaState)> {
        // Extract score from event
        let score = event
            .score
            .context("Event has no score field")?
            as f64;

        tracing::trace!(
            score = score,
            current_ema = ?current_state.as_ref().map(|s| s.ema),
            "Evaluating EMA condition"
        );

        // Calculate new EMA
        let new_ema = match current_state {
            Some(state) => {
                // EMA formula: EMA_new = alpha * score + (1 - alpha) * EMA_old
                self.alpha * score + (1.0 - self.alpha) * state.ema
            }
            None => {
                // First value: EMA = score
                score
            }
        };

        let new_count = current_state.map(|s| s.count).unwrap_or(0) + 1;

        // Create new state
        let new_state = EmaState {
            ema: new_ema,
            count: new_count,
            last_updated: Utc::now(),
        };

        // Extract threshold and operator from condition
        let threshold = condition
            .value
            .parse::<f64>()
            .with_context(|| format!("Invalid threshold value: {}", condition.value))?;

        let operator = condition.operator.as_str();

        // Evaluate condition
        let matches = match operator {
            "<" => new_ema < threshold,
            ">" => new_ema > threshold,
            "<=" => new_ema <= threshold,
            ">=" => new_ema >= threshold,
            "=" | "==" => (new_ema - threshold).abs() < f64::EPSILON,
            "!=" | "<>" => (new_ema - threshold).abs() >= f64::EPSILON,
            _ => anyhow::bail!("Invalid operator: {}", operator),
        };

        tracing::debug!(
            new_ema = new_ema,
            threshold = threshold,
            operator = operator,
            matches = matches,
            count = new_count,
            "EMA evaluation complete"
        );

        Ok((matches, new_state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test event with customizable score
    fn create_test_event(score: i32) -> Event {
        Event {
            id: "test-event".to_string(),
            chain_id: 84532,
            block_number: 1000,
            block_hash: "0xabc".to_string(),
            transaction_hash: "0xdef".to_string(),
            log_index: 0,
            registry: "reputation".to_string(),
            event_type: "NewFeedback".to_string(),
            agent_id: Some(42),
            timestamp: 1234567890,
            owner: None,
            token_uri: None,
            metadata_key: None,
            metadata_value: None,
            client_address: Some("0x123".to_string()),
            feedback_index: Some(0),
            score: Some(score),
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
            condition_type: "ema_threshold".to_string(),
            field: "score".to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
            config: Some(serde_json::json!({ "window_size": 10 })),
            created_at: Utc::now(),
        }
    }

    // ========================================================================
    // Constructor tests
    // ========================================================================

    #[test]
    fn test_ema_new_calculates_alpha_correctly() {
        let evaluator = EmaEvaluator::new(10);
        // alpha = 2 / (10 + 1) ≈ 0.1818
        assert!((evaluator.alpha - 0.1818).abs() < 0.001);
    }

    #[test]
    fn test_ema_new_window_size_1() {
        let evaluator = EmaEvaluator::new(1);
        // alpha = 2 / (1 + 1) = 1.0 (full weight to new value)
        assert!((evaluator.alpha - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ema_from_config_valid() {
        let config = serde_json::json!({ "window_size": 20 });
        let evaluator = EmaEvaluator::from_config(&config).unwrap();
        assert_eq!(evaluator.window_size, 20);
        // alpha = 2 / 21 ≈ 0.0952
        assert!((evaluator.alpha - 0.0952).abs() < 0.001);
    }

    #[test]
    fn test_ema_from_config_missing_window_size() {
        let config = serde_json::json!({});
        let result = EmaEvaluator::from_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("window_size"));
    }

    #[test]
    fn test_ema_from_config_zero_window_size() {
        let config = serde_json::json!({ "window_size": 0 });
        let result = EmaEvaluator::from_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("greater than 0"));
    }

    // ========================================================================
    // First value tests
    // ========================================================================

    #[test]
    fn test_ema_first_value_equals_score() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(85);
        let condition = create_test_condition("<", "90");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();

        assert_eq!(state.ema, 85.0);
        assert_eq!(state.count, 1);
        assert!(matches); // 85 < 90
    }

    #[test]
    fn test_ema_first_value_negative_score() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(-10);
        let condition = create_test_condition("<", "0");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();

        assert_eq!(state.ema, -10.0);
        assert_eq!(state.count, 1);
        assert!(matches); // -10 < 0
    }

    #[test]
    fn test_ema_first_value_zero_score() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(0);
        let condition = create_test_condition("=", "0");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();

        assert_eq!(state.ema, 0.0);
        assert_eq!(state.count, 1);
        assert!(matches); // 0 == 0
    }

    // ========================================================================
    // EMA calculation tests
    // ========================================================================

    #[test]
    fn test_ema_calculation_window_10() {
        let evaluator = EmaEvaluator::new(10); // alpha ≈ 0.1818
        let event = create_test_event(90);
        let condition = create_test_condition("<", "75");

        let initial_state = EmaState {
            ema: 75.0,
            count: 5,
            last_updated: Utc::now(),
        };

        let (matches, state) = evaluator
            .evaluate(&event, &condition, Some(initial_state))
            .unwrap();

        // EMA = 0.1818 * 90 + 0.8182 * 75 ≈ 77.73
        assert!((state.ema - 77.727).abs() < 0.01);
        assert_eq!(state.count, 6);
        assert!(!matches); // 77.73 is NOT < 75
    }

    #[test]
    fn test_ema_calculation_window_1_full_weight() {
        let evaluator = EmaEvaluator::new(1); // alpha = 1.0
        let event = create_test_event(100);
        let condition = create_test_condition(">", "90");

        let initial_state = EmaState {
            ema: 50.0,
            count: 1,
            last_updated: Utc::now(),
        };

        let (matches, state) = evaluator
            .evaluate(&event, &condition, Some(initial_state))
            .unwrap();

        // EMA = 1.0 * 100 + 0.0 * 50 = 100 (new value completely replaces old)
        assert_eq!(state.ema, 100.0);
        assert_eq!(state.count, 2);
        assert!(matches); // 100 > 90
    }

    #[test]
    fn test_ema_series_converges() {
        let evaluator = EmaEvaluator::new(5); // alpha = 2/6 ≈ 0.333
        let condition = create_test_condition(">", "80");

        // Start with EMA = 50
        let mut state = EmaState {
            ema: 50.0,
            count: 0,
            last_updated: Utc::now(),
        };

        // Feed 10 events with score = 100
        for i in 0..10 {
            let event = create_test_event(100);
            let (_, new_state) = evaluator
                .evaluate(&event, &condition, Some(state.clone()))
                .unwrap();
            state = new_state;

            tracing::debug!("Iteration {}: EMA = {}", i, state.ema);
        }

        // EMA should converge towards 100
        assert!(state.ema > 95.0); // After 10 iterations, should be close to 100
        assert!(state.ema <= 100.0);
    }

    // ========================================================================
    // Operator tests
    // ========================================================================

    #[test]
    fn test_ema_operator_less_than_match() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(50);
        let condition = create_test_condition("<", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 50 < 60
    }

    #[test]
    fn test_ema_operator_less_than_no_match() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(70);
        let condition = create_test_condition("<", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(!matches); // 70 is NOT < 60
    }

    #[test]
    fn test_ema_operator_greater_than_match() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(90);
        let condition = create_test_condition(">", "80");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 90 > 80
    }

    #[test]
    fn test_ema_operator_less_or_equal_match_equal() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let condition = create_test_condition("<=", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 60 <= 60
    }

    #[test]
    fn test_ema_operator_less_or_equal_match_less() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(50);
        let condition = create_test_condition("<=", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 50 <= 60
    }

    #[test]
    fn test_ema_operator_greater_or_equal_match() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(80);
        let condition = create_test_condition(">=", "80");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 80 >= 80
    }

    #[test]
    fn test_ema_operator_equals_match() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let condition = create_test_condition("=", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 60 == 60
    }

    #[test]
    fn test_ema_operator_equals_double_equals() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let condition = create_test_condition("==", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 60 == 60
    }

    #[test]
    fn test_ema_operator_not_equals_match() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(70);
        let condition = create_test_condition("!=", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 70 != 60
    }

    #[test]
    fn test_ema_operator_not_equals_diamond() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(70);
        let condition = create_test_condition("<>", "60");

        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches); // 70 <> 60
    }

    #[test]
    fn test_ema_operator_invalid() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let condition = create_test_condition("~", "60");

        let result = evaluator.evaluate(&event, &condition, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid operator"));
    }

    // ========================================================================
    // Error handling tests
    // ========================================================================

    #[test]
    fn test_ema_event_without_score() {
        let evaluator = EmaEvaluator::new(10);
        let mut event = create_test_event(60);
        event.score = None;
        let condition = create_test_condition("<", "70");

        let result = evaluator.evaluate(&event, &condition, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no score field"));
    }

    #[test]
    fn test_ema_invalid_threshold_value() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let mut condition = create_test_condition("<", "not_a_number");
        condition.value = "not_a_number".to_string();

        let result = evaluator.evaluate(&event, &condition, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid threshold"));
    }

    // ========================================================================
    // Edge case tests
    // ========================================================================

    #[test]
    fn test_ema_very_high_score() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(i32::MAX);
        let condition = create_test_condition(">", "1000000");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches);
        assert_eq!(state.ema, i32::MAX as f64);
    }

    #[test]
    fn test_ema_very_low_score() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(i32::MIN);
        let condition = create_test_condition("<", "-1000000");

        let (matches, state) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(matches);
        assert_eq!(state.ema, i32::MIN as f64);
    }

    #[test]
    fn test_ema_floating_point_precision() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let condition = create_test_condition("=", "60.0000000001");

        // Should not match due to floating point precision
        let (matches, _) = evaluator.evaluate(&event, &condition, None).unwrap();
        assert!(!matches);
    }

    #[test]
    fn test_ema_state_timestamp_updated() {
        let evaluator = EmaEvaluator::new(10);
        let event = create_test_event(60);
        let condition = create_test_condition("<", "70");

        let old_timestamp = Utc::now() - chrono::Duration::hours(1);
        let initial_state = EmaState {
            ema: 50.0,
            count: 5,
            last_updated: old_timestamp,
        };

        let (_, new_state) = evaluator
            .evaluate(&event, &condition, Some(initial_state))
            .unwrap();

        assert!(new_state.last_updated > old_timestamp);
    }

    // ========================================================================
    // State serialization tests
    // ========================================================================

    #[test]
    fn test_ema_state_serialization() {
        let state = EmaState {
            ema: 75.5,
            count: 10,
            last_updated: Utc::now(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: EmaState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.ema, deserialized.ema);
        assert_eq!(state.count, deserialized.count);
    }
}
