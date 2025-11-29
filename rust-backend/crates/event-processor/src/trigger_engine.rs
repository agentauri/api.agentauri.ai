//! Trigger matching engine
//!
//! Evaluates events against user-defined trigger conditions.
//!
//! # Stateless Conditions (Phase 3)
//! - agent_id_equals: Match specific agent ID
//! - score_threshold: Compare score with threshold
//! - tag_equals: Match tag1 or tag2
//! - event_type_equals: Match event type
//!
//! # Stateful Conditions (Week 14)
//! - ema_threshold: Exponential moving average of scores
//! - rate_limit: Event count in sliding time window

use anyhow::{bail, Context, Result};
use shared::models::{Event, Trigger, TriggerCondition};

use crate::evaluators::{EmaEvaluator, EmaState, RateCounterEvaluator, RateCounterState};
use crate::state_manager::TriggerStateManager;

/// Supported condition types
pub mod condition_types {
    // Stateless conditions
    pub const AGENT_ID_EQUALS: &str = "agent_id_equals";
    pub const SCORE_THRESHOLD: &str = "score_threshold";
    pub const TAG_EQUALS: &str = "tag_equals";
    pub const EVENT_TYPE_EQUALS: &str = "event_type_equals";

    // Stateful conditions
    pub const EMA_THRESHOLD: &str = "ema_threshold";
    pub const RATE_LIMIT: &str = "rate_limit";
}

/// Evaluate a single condition against an event
///
/// # Arguments
///
/// * `condition` - The condition to evaluate
/// * `event` - The event to evaluate against
///
/// # Returns
///
/// `true` if the condition matches, `false` otherwise
pub fn evaluate_condition(condition: &TriggerCondition, event: &Event) -> Result<bool> {
    let result = match condition.condition_type.as_str() {
        condition_types::AGENT_ID_EQUALS => evaluate_agent_id_equals(condition, event),
        condition_types::SCORE_THRESHOLD => evaluate_score_threshold(condition, event),
        condition_types::TAG_EQUALS => evaluate_tag_equals(condition, event),
        condition_types::EVENT_TYPE_EQUALS => evaluate_event_type_equals(condition, event),
        unknown => bail!("Unknown condition type: {}", unknown),
    };

    result.with_context(|| {
        format!(
            "Failed evaluating condition id={} type={} for trigger={}",
            condition.id, condition.condition_type, condition.trigger_id
        )
    })
}

/// Evaluate agent_id_equals condition
///
/// Matches when event.agent_id equals the condition value
fn evaluate_agent_id_equals(condition: &TriggerCondition, event: &Event) -> Result<bool> {
    let target_agent_id: i64 = condition
        .value
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid agent_id value: {}", condition.value))?;

    match event.agent_id {
        Some(agent_id) => Ok(agent_id == target_agent_id),
        None => {
            tracing::trace!(
                condition_type = "agent_id_equals",
                "Field is None, returning false"
            );
            Ok(false)
        }
    }
}

/// Evaluate score_threshold condition
///
/// Compares event.score against the condition value using the operator
/// Supported operators: <, >, =, <=, >=, !=
fn evaluate_score_threshold(condition: &TriggerCondition, event: &Event) -> Result<bool> {
    let threshold: i32 = condition
        .value
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid score threshold value: {}", condition.value))?;

    let score = match event.score {
        Some(s) => s,
        None => {
            tracing::trace!(
                condition_type = "score_threshold",
                "Field is None, returning false"
            );
            return Ok(false);
        }
    };

    let result = match condition.operator.as_str() {
        "<" => score < threshold,
        ">" => score > threshold,
        "=" | "==" => score == threshold,
        "<=" => score <= threshold,
        ">=" => score >= threshold,
        "!=" | "<>" => score != threshold,
        op => bail!("Invalid score_threshold operator: {}", op),
    };

    Ok(result)
}

/// Evaluate tag_equals condition
///
/// Matches when the specified tag field equals the condition value
/// Field can be "tag1" or "tag2"
fn evaluate_tag_equals(condition: &TriggerCondition, event: &Event) -> Result<bool> {
    let tag_value = match condition.field.as_str() {
        "tag1" => &event.tag1,
        "tag2" => &event.tag2,
        field => bail!("Invalid tag field: {} (expected 'tag1' or 'tag2')", field),
    };

    match tag_value {
        Some(tag) => Ok(tag == &condition.value),
        None => {
            tracing::trace!(
                condition_type = "tag_equals",
                "Field is None, returning false"
            );
            Ok(false)
        }
    }
}

/// Evaluate event_type_equals condition
///
/// Matches when event.event_type equals the condition value
fn evaluate_event_type_equals(condition: &TriggerCondition, event: &Event) -> Result<bool> {
    Ok(event.event_type == condition.value)
}

/// Evaluate all conditions against an event (AND logic) - STATELESS ONLY
///
/// # Arguments
///
/// * `conditions` - List of conditions to evaluate
/// * `event` - The event to evaluate against
///
/// # Returns
///
/// `true` if ALL conditions match, `false` if any condition fails
///
/// # Note
///
/// This function only handles stateless conditions. For stateful triggers,
/// use `evaluate_trigger_stateful` instead.
pub fn evaluate_trigger(conditions: &[TriggerCondition], event: &Event) -> Result<bool> {
    // Empty conditions list matches all events
    if conditions.is_empty() {
        tracing::warn!("Trigger has no conditions - will match ALL events");
        return Ok(true);
    }

    for condition in conditions {
        let matches = evaluate_condition(condition, event)?;
        if !matches {
            tracing::debug!(
                condition_id = condition.id,
                condition_type = %condition.condition_type,
                "Condition did not match"
            );
            return Ok(false);
        }
    }

    tracing::debug!("All {} conditions matched", conditions.len());
    Ok(true)
}

/// Evaluate all conditions against an event with state management (AND logic)
///
/// # Arguments
///
/// * `trigger` - The trigger being evaluated
/// * `conditions` - List of conditions to evaluate
/// * `event` - The event to evaluate against
/// * `state_manager` - State manager for loading/updating stateful condition state
///
/// # Returns
///
/// `true` if ALL conditions match, `false` if any condition fails
///
/// # Errors
///
/// Returns error if:
/// - Condition evaluation fails
/// - State loading/updating fails
/// - Invalid condition configuration
pub async fn evaluate_trigger_stateful(
    trigger: &Trigger,
    conditions: &[TriggerCondition],
    event: &Event,
    state_manager: &TriggerStateManager,
) -> Result<bool> {
    // Empty conditions list matches all events
    if conditions.is_empty() {
        tracing::warn!(
            trigger_id = %trigger.id,
            "Trigger has no conditions - will match ALL events"
        );
        return Ok(true);
    }

    // Load current state if trigger is stateful
    let current_state = if trigger.is_stateful {
        state_manager
            .load_state(&trigger.id)
            .await
            .with_context(|| format!("Failed to load state for trigger {}", trigger.id))?
    } else {
        None
    };

    // Track if we need to update state
    let mut new_state: Option<serde_json::Value> = None;

    // Evaluate each condition
    for condition in conditions {
        let matches = match condition.condition_type.as_str() {
            condition_types::EMA_THRESHOLD => {
                // EMA evaluation
                let config = condition
                    .config
                    .as_ref()
                    .context("EMA condition missing config")?;

                let evaluator = EmaEvaluator::from_config(config)
                    .with_context(|| format!("Invalid EMA config for condition {}", condition.id))?;

                // Extract EMA state from current_state
                let ema_state = current_state
                    .as_ref()
                    .and_then(|s| serde_json::from_value::<EmaState>(s.clone()).ok());

                let (condition_matches, updated_state) =
                    evaluator.evaluate(event, condition, ema_state)?;

                // Store updated state for persistence
                new_state = Some(serde_json::to_value(updated_state)?);

                condition_matches
            }
            condition_types::RATE_LIMIT => {
                // Rate counter evaluation
                let config = condition
                    .config
                    .as_ref()
                    .context("Rate limit condition missing config")?;

                let evaluator = RateCounterEvaluator::from_config(config).with_context(|| {
                    format!("Invalid rate counter config for condition {}", condition.id)
                })?;

                // Extract rate counter state from current_state
                let counter_state = current_state
                    .as_ref()
                    .and_then(|s| serde_json::from_value::<RateCounterState>(s.clone()).ok());

                let (condition_matches, updated_state) =
                    evaluator.evaluate(event, condition, counter_state)?;

                // Store updated state for persistence
                new_state = Some(serde_json::to_value(updated_state)?);

                condition_matches
            }
            // Stateless conditions
            _ => evaluate_condition(condition, event)?,
        };

        if !matches {
            tracing::debug!(
                trigger_id = %trigger.id,
                condition_id = condition.id,
                condition_type = %condition.condition_type,
                "Condition did not match"
            );

            // Still update state even if condition doesn't match
            // (state should reflect all events, not just matches)
            if let Some(state) = new_state {
                state_manager
                    .update_state(&trigger.id, state)
                    .await
                    .with_context(|| {
                        format!("Failed to update state for trigger {}", trigger.id)
                    })?;
            }

            return Ok(false);
        }
    }

    // All conditions matched - update state
    if let Some(state) = new_state {
        state_manager
            .update_state(&trigger.id, state)
            .await
            .with_context(|| format!("Failed to update state for trigger {}", trigger.id))?;

        tracing::debug!(
            trigger_id = %trigger.id,
            "Updated trigger state after successful match"
        );
    }

    tracing::debug!(
        trigger_id = %trigger.id,
        conditions_count = conditions.len(),
        "All conditions matched"
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Create a test event with customizable fields
    fn create_test_event() -> Event {
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
    fn create_condition(
        condition_type: &str,
        field: &str,
        operator: &str,
        value: &str,
    ) -> TriggerCondition {
        TriggerCondition {
            id: 1,
            trigger_id: "test-trigger".to_string(),
            condition_type: condition_type.to_string(),
            field: field.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
            config: None,
            created_at: Utc::now(),
        }
    }

    // ========================================================================
    // agent_id_equals tests
    // ========================================================================

    #[test]
    fn test_agent_id_equals_match() {
        let event = create_test_event();
        let condition = create_condition("agent_id_equals", "agent_id", "=", "42");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_agent_id_equals_no_match() {
        let event = create_test_event();
        let condition = create_condition("agent_id_equals", "agent_id", "=", "99");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_agent_id_equals_none() {
        let mut event = create_test_event();
        event.agent_id = None;
        let condition = create_condition("agent_id_equals", "agent_id", "=", "42");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_agent_id_equals_invalid_value() {
        let event = create_test_event();
        let condition = create_condition("agent_id_equals", "agent_id", "=", "not_a_number");

        assert!(evaluate_condition(&condition, &event).is_err());
    }

    // ========================================================================
    // score_threshold tests
    // ========================================================================

    #[test]
    fn test_score_threshold_less_than_match() {
        let mut event = create_test_event();
        event.score = Some(50);
        let condition = create_condition("score_threshold", "score", "<", "60");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_less_than_no_match() {
        let mut event = create_test_event();
        event.score = Some(70);
        let condition = create_condition("score_threshold", "score", "<", "60");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_greater_than_match() {
        let mut event = create_test_event();
        event.score = Some(90);
        let condition = create_condition("score_threshold", "score", ">", "80");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_greater_than_no_match() {
        let mut event = create_test_event();
        event.score = Some(70);
        let condition = create_condition("score_threshold", "score", ">", "80");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_equals_match() {
        let mut event = create_test_event();
        event.score = Some(60);
        let condition = create_condition("score_threshold", "score", "=", "60");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_equals_no_match() {
        let mut event = create_test_event();
        event.score = Some(61);
        let condition = create_condition("score_threshold", "score", "=", "60");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_less_or_equal_match_equal() {
        let mut event = create_test_event();
        event.score = Some(60);
        let condition = create_condition("score_threshold", "score", "<=", "60");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_less_or_equal_match_less() {
        let mut event = create_test_event();
        event.score = Some(50);
        let condition = create_condition("score_threshold", "score", "<=", "60");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_greater_or_equal_match() {
        let mut event = create_test_event();
        event.score = Some(80);
        let condition = create_condition("score_threshold", "score", ">=", "80");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_not_equal_match() {
        let mut event = create_test_event();
        event.score = Some(70);
        let condition = create_condition("score_threshold", "score", "!=", "60");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_none() {
        let mut event = create_test_event();
        event.score = None;
        let condition = create_condition("score_threshold", "score", "<", "60");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_score_threshold_invalid_operator() {
        let event = create_test_event();
        let condition = create_condition("score_threshold", "score", "~", "60");

        assert!(evaluate_condition(&condition, &event).is_err());
    }

    #[test]
    fn test_score_threshold_invalid_value() {
        let event = create_test_event();
        let condition = create_condition("score_threshold", "score", "<", "not_a_number");

        assert!(evaluate_condition(&condition, &event).is_err());
    }

    // ========================================================================
    // tag_equals tests
    // ========================================================================

    #[test]
    fn test_tag_equals_tag1_match() {
        let event = create_test_event();
        let condition = create_condition("tag_equals", "tag1", "=", "trade");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_tag_equals_tag1_no_match() {
        let event = create_test_event();
        let condition = create_condition("tag_equals", "tag1", "=", "other");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_tag_equals_tag2_match() {
        let event = create_test_event();
        let condition = create_condition("tag_equals", "tag2", "=", "reliable");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_tag_equals_tag2_no_match() {
        let event = create_test_event();
        let condition = create_condition("tag_equals", "tag2", "=", "other");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_tag_equals_tag1_none() {
        let mut event = create_test_event();
        event.tag1 = None;
        let condition = create_condition("tag_equals", "tag1", "=", "trade");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_tag_equals_invalid_field() {
        let event = create_test_event();
        let condition = create_condition("tag_equals", "tag3", "=", "trade");

        assert!(evaluate_condition(&condition, &event).is_err());
    }

    // ========================================================================
    // event_type_equals tests
    // ========================================================================

    #[test]
    fn test_event_type_equals_match() {
        let event = create_test_event();
        let condition = create_condition("event_type_equals", "event_type", "=", "NewFeedback");

        assert!(evaluate_condition(&condition, &event).unwrap());
    }

    #[test]
    fn test_event_type_equals_no_match() {
        let event = create_test_event();
        let condition = create_condition("event_type_equals", "event_type", "=", "FeedbackRevoked");

        assert!(!evaluate_condition(&condition, &event).unwrap());
    }

    // ========================================================================
    // evaluate_trigger tests (AND logic)
    // ========================================================================

    #[test]
    fn test_evaluate_trigger_all_conditions_match() {
        let event = create_test_event();
        let conditions = vec![
            create_condition("agent_id_equals", "agent_id", "=", "42"),
            create_condition("score_threshold", "score", ">", "80"),
            create_condition("tag_equals", "tag1", "=", "trade"),
        ];

        assert!(evaluate_trigger(&conditions, &event).unwrap());
    }

    #[test]
    fn test_evaluate_trigger_one_condition_fails() {
        let event = create_test_event();
        let conditions = vec![
            create_condition("agent_id_equals", "agent_id", "=", "42"),
            create_condition("score_threshold", "score", ">", "90"), // score is 85, fails
            create_condition("tag_equals", "tag1", "=", "trade"),
        ];

        assert!(!evaluate_trigger(&conditions, &event).unwrap());
    }

    #[test]
    fn test_evaluate_trigger_empty_conditions_matches() {
        let event = create_test_event();
        let conditions: Vec<TriggerCondition> = vec![];

        assert!(evaluate_trigger(&conditions, &event).unwrap());
    }

    #[test]
    fn test_evaluate_trigger_first_condition_fails() {
        let event = create_test_event();
        let conditions = vec![
            create_condition("agent_id_equals", "agent_id", "=", "99"), // fails
            create_condition("score_threshold", "score", ">", "80"),
        ];

        assert!(!evaluate_trigger(&conditions, &event).unwrap());
    }

    #[test]
    fn test_evaluate_trigger_error_in_middle_condition() {
        let event = create_test_event();
        let conditions = vec![
            create_condition("agent_id_equals", "agent_id", "=", "42"), // valid, matches
            create_condition("score_threshold", "score", "~", "60"),    // invalid operator
            create_condition("tag_equals", "tag1", "=", "trade"),       // never evaluated
        ];

        // Should propagate the error from the invalid condition
        assert!(evaluate_trigger(&conditions, &event).is_err());
    }

    // ========================================================================
    // Unknown condition type test
    // ========================================================================

    #[test]
    fn test_unknown_condition_type() {
        let event = create_test_event();
        let condition = create_condition("unknown_type", "field", "=", "value");

        assert!(evaluate_condition(&condition, &event).is_err());
    }
}
