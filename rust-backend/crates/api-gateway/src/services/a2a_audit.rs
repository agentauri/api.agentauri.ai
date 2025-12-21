//! A2A Task Audit Service
//!
//! Provides audit logging for A2A Protocol task operations.
//! Records all task lifecycle events for security, compliance, and analytics.

use anyhow::{Context, Result};
use shared::DbPool;
use uuid::Uuid;

/// Event types for audit logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    Created,
    Started,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::Created => "created",
            AuditEventType::Started => "started",
            AuditEventType::Completed => "completed",
            AuditEventType::Failed => "failed",
            AuditEventType::Cancelled => "cancelled",
            AuditEventType::Timeout => "timeout",
        }
    }
}

/// Actor types for audit logging
#[derive(Debug, Clone)]
pub enum AuditActor {
    User(String),
    System,
    ApiKey(String),
}

impl AuditActor {
    pub fn actor_type(&self) -> &'static str {
        match self {
            AuditActor::User(_) => "user",
            AuditActor::System => "system",
            AuditActor::ApiKey(_) => "api_key",
        }
    }

    pub fn actor_id(&self) -> Option<&str> {
        match self {
            AuditActor::User(id) => Some(id),
            AuditActor::System => Some("processor"),
            AuditActor::ApiKey(prefix) => Some(prefix),
        }
    }
}

/// Parameters for creating an audit log entry
pub struct AuditLogParams<'a> {
    pub task_id: &'a Uuid,
    pub organization_id: &'a str,
    pub event_type: AuditEventType,
    pub actor: AuditActor,
    pub tool: Option<&'a str>,
    pub cost_micro_usdc: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<&'a str>,
    pub metadata: Option<serde_json::Value>,
}

/// A2A Audit Service
pub struct A2aAuditService;

impl A2aAuditService {
    /// Log an audit event
    pub async fn log(pool: &DbPool, params: AuditLogParams<'_>) -> Result<Uuid> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO a2a_task_audit_log (
                task_id, organization_id, event_type,
                actor_type, actor_id, tool,
                cost_micro_usdc, duration_ms, error_message,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(params.task_id)
        .bind(params.organization_id)
        .bind(params.event_type.as_str())
        .bind(params.actor.actor_type())
        .bind(params.actor.actor_id())
        .bind(params.tool)
        .bind(params.cost_micro_usdc)
        .bind(params.duration_ms)
        .bind(params.error_message)
        .bind(params.metadata.unwrap_or(serde_json::json!({})))
        .fetch_one(pool)
        .await
        .context("Failed to create audit log entry")?;

        Ok(id)
    }

    /// Log task creation
    pub async fn log_created(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
        tool: &str,
        actor: AuditActor,
    ) -> Result<Uuid> {
        Self::log(
            pool,
            AuditLogParams {
                task_id,
                organization_id,
                event_type: AuditEventType::Created,
                actor,
                tool: Some(tool),
                cost_micro_usdc: None,
                duration_ms: None,
                error_message: None,
                metadata: None,
            },
        )
        .await
    }

    /// Log task started
    pub async fn log_started(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
        tool: &str,
    ) -> Result<Uuid> {
        Self::log(
            pool,
            AuditLogParams {
                task_id,
                organization_id,
                event_type: AuditEventType::Started,
                actor: AuditActor::System,
                tool: Some(tool),
                cost_micro_usdc: None,
                duration_ms: None,
                error_message: None,
                metadata: None,
            },
        )
        .await
    }

    /// Log task completed
    pub async fn log_completed(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
        tool: &str,
        cost_micro_usdc: i64,
        duration_ms: i64,
    ) -> Result<Uuid> {
        Self::log(
            pool,
            AuditLogParams {
                task_id,
                organization_id,
                event_type: AuditEventType::Completed,
                actor: AuditActor::System,
                tool: Some(tool),
                cost_micro_usdc: Some(cost_micro_usdc),
                duration_ms: Some(duration_ms),
                error_message: None,
                metadata: None,
            },
        )
        .await
    }

    /// Log task failed
    pub async fn log_failed(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
        tool: &str,
        duration_ms: i64,
        error_message: &str,
    ) -> Result<Uuid> {
        Self::log(
            pool,
            AuditLogParams {
                task_id,
                organization_id,
                event_type: AuditEventType::Failed,
                actor: AuditActor::System,
                tool: Some(tool),
                cost_micro_usdc: None,
                duration_ms: Some(duration_ms),
                error_message: Some(error_message),
                metadata: None,
            },
        )
        .await
    }

    /// Log task timeout
    pub async fn log_timeout(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
        tool: &str,
        duration_ms: i64,
    ) -> Result<Uuid> {
        Self::log(
            pool,
            AuditLogParams {
                task_id,
                organization_id,
                event_type: AuditEventType::Timeout,
                actor: AuditActor::System,
                tool: Some(tool),
                cost_micro_usdc: None,
                duration_ms: Some(duration_ms),
                error_message: Some("Query execution timeout"),
                metadata: None,
            },
        )
        .await
    }

    /// Log task cancelled
    pub async fn log_cancelled(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
        actor: AuditActor,
    ) -> Result<Uuid> {
        Self::log(
            pool,
            AuditLogParams {
                task_id,
                organization_id,
                event_type: AuditEventType::Cancelled,
                actor,
                tool: None,
                cost_micro_usdc: None,
                duration_ms: None,
                error_message: None,
                metadata: None,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_type_as_str() {
        assert_eq!(AuditEventType::Created.as_str(), "created");
        assert_eq!(AuditEventType::Started.as_str(), "started");
        assert_eq!(AuditEventType::Completed.as_str(), "completed");
        assert_eq!(AuditEventType::Failed.as_str(), "failed");
        assert_eq!(AuditEventType::Cancelled.as_str(), "cancelled");
        assert_eq!(AuditEventType::Timeout.as_str(), "timeout");
    }

    #[test]
    fn test_audit_actor() {
        let user = AuditActor::User("user-123".to_string());
        assert_eq!(user.actor_type(), "user");
        assert_eq!(user.actor_id(), Some("user-123"));

        let system = AuditActor::System;
        assert_eq!(system.actor_type(), "system");
        assert_eq!(system.actor_id(), Some("processor"));

        let api_key = AuditActor::ApiKey("sk_live_abc".to_string());
        assert_eq!(api_key.actor_type(), "api_key");
        assert_eq!(api_key.actor_id(), Some("sk_live_abc"));
    }
}
