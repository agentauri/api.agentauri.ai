//! Action job definitions for event processor and action workers
//!
//! Jobs are created when triggers match and are enqueued to Redis for action workers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Queue name for action jobs
pub const ACTION_JOBS_QUEUE: &str = "action_jobs";

/// Dead letter queue for failed jobs
pub const ACTION_JOBS_DLQ: &str = "action_jobs_dlq";

/// Action type enum for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Telegram,
    Rest,
    Mcp,
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ActionType::Telegram => "telegram",
            ActionType::Rest => "rest",
            ActionType::Mcp => "mcp",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ActionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "telegram" => Ok(ActionType::Telegram),
            "rest" => Ok(ActionType::Rest),
            "mcp" => Ok(ActionType::Mcp),
            _ => anyhow::bail!("Invalid action type: {}", s),
        }
    }
}

/// Action job to be processed by action workers
///
/// Jobs are created when a trigger matches an event and contain all
/// information needed by action workers to execute the action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionJob {
    /// Unique job identifier
    pub id: String,
    /// ID of the trigger that created this job
    pub trigger_id: String,
    /// ID of the event that triggered this job
    pub event_id: String,
    /// Type of action to execute
    pub action_type: ActionType,
    /// Priority for queue ordering (higher = more urgent)
    pub priority: i32,
    /// Action-specific configuration
    pub config: serde_json::Value,
    /// Event data for template variable substitution
    /// Contains flattened event fields: agent_id, score, chain_id, event_type, etc.
    pub event_data: serde_json::Value,
    /// When this job was created
    pub created_at: DateTime<Utc>,
}

impl ActionJob {
    /// Create a new action job
    ///
    /// # Arguments
    ///
    /// * `trigger_id` - ID of the trigger that matched
    /// * `event_id` - ID of the event that triggered this action
    /// * `action_type` - Type of action
    /// * `priority` - Job priority (higher = more urgent)
    /// * `config` - Action-specific configuration
    /// * `event_data` - Event data for template variable substitution
    pub fn new(
        trigger_id: &str,
        event_id: &str,
        action_type: ActionType,
        priority: i32,
        config: serde_json::Value,
        event_data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            trigger_id: trigger_id.to_string(),
            event_id: event_id.to_string(),
            action_type,
            priority,
            config,
            event_data,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_action_job_new() {
        let config = json!({
            "chat_id": "123456789",
            "message_template": "Test message"
        });
        let event_data = json!({
            "agent_id": 42,
            "score": 85,
            "chain_id": 11155111
        });

        let job = ActionJob::new(
            "trigger-123",
            "event-456",
            ActionType::Telegram,
            1,
            config.clone(),
            event_data.clone(),
        );

        assert!(!job.id.is_empty());
        assert_eq!(job.trigger_id, "trigger-123");
        assert_eq!(job.event_id, "event-456");
        assert_eq!(job.action_type, ActionType::Telegram);
        assert_eq!(job.priority, 1);
        assert_eq!(job.config, config);
        assert_eq!(job.event_data, event_data);
    }

    #[test]
    fn test_action_job_serialization() {
        let config = json!({"url": "https://example.com"});
        let event_data = json!({"agent_id": 42});
        let job = ActionJob::new("t1", "e1", ActionType::Rest, 2, config, event_data);

        let serialized = serde_json::to_string(&job).unwrap();
        let deserialized: ActionJob = serde_json::from_str(&serialized).unwrap();

        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.trigger_id, deserialized.trigger_id);
        assert_eq!(job.event_id, deserialized.event_id);
        assert_eq!(job.action_type, deserialized.action_type);
        assert_eq!(job.priority, deserialized.priority);
        assert_eq!(job.event_data, deserialized.event_data);
        assert_eq!(job.created_at, deserialized.created_at);
    }

    #[test]
    fn test_action_job_ids_are_unique() {
        let config = json!({"key": "value"});
        let event_data = json!({"agent_id": 42});
        let job1 = ActionJob::new(
            "t1",
            "e1",
            ActionType::Telegram,
            1,
            config.clone(),
            event_data.clone(),
        );
        let job2 = ActionJob::new("t1", "e1", ActionType::Telegram, 1, config, event_data);

        // UUIDs should be unique even for identical parameters
        assert_ne!(job1.id, job2.id);
    }

    #[test]
    fn test_action_type_display() {
        assert_eq!(ActionType::Telegram.to_string(), "telegram");
        assert_eq!(ActionType::Rest.to_string(), "rest");
        assert_eq!(ActionType::Mcp.to_string(), "mcp");
    }

    #[test]
    fn test_action_type_from_str() {
        assert_eq!(
            "telegram".parse::<ActionType>().unwrap(),
            ActionType::Telegram
        );
        assert_eq!("rest".parse::<ActionType>().unwrap(), ActionType::Rest);
        assert_eq!("mcp".parse::<ActionType>().unwrap(), ActionType::Mcp);
        assert!("invalid".parse::<ActionType>().is_err());
    }

    #[test]
    fn test_action_type_case_insensitive() {
        assert_eq!(
            "TELEGRAM".parse::<ActionType>().unwrap(),
            ActionType::Telegram
        );
        assert_eq!("REST".parse::<ActionType>().unwrap(), ActionType::Rest);
        assert_eq!("Mcp".parse::<ActionType>().unwrap(), ActionType::Mcp);
    }
}
