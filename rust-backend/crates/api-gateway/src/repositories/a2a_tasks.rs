//! A2A Tasks repository for database operations
//!
//! Handles CRUD operations for async tasks submitted via the A2A JSON-RPC protocol.

use anyhow::{Context, Result};
use shared::DbPool;
use uuid::Uuid;

use crate::models::a2a::A2aTask;

pub struct A2aTaskRepository;

impl A2aTaskRepository {
    /// Create a new task
    pub async fn create(
        pool: &DbPool,
        organization_id: &str,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> Result<A2aTask> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            INSERT INTO a2a_tasks (organization_id, tool, arguments, status)
            VALUES ($1, $2, $3, 'submitted')
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(tool)
        .bind(arguments)
        .fetch_one(pool)
        .await
        .context("Failed to create A2A task")?;

        Ok(task)
    }

    /// Find task by ID
    pub async fn find_by_id(pool: &DbPool, task_id: &Uuid) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            SELECT * FROM a2a_tasks
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find A2A task by ID")?;

        Ok(task)
    }

    /// Find task by ID and organization (for authorization)
    pub async fn find_by_id_and_org(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
    ) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            SELECT * FROM a2a_tasks
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(task_id)
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find A2A task")?;

        Ok(task)
    }

    /// Update task status to 'working' and set started_at
    pub async fn start_task(pool: &DbPool, task_id: &Uuid) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            UPDATE a2a_tasks
            SET status = 'working', started_at = NOW()
            WHERE id = $1 AND status = 'submitted'
            RETURNING *
            "#,
        )
        .bind(task_id)
        .fetch_optional(pool)
        .await
        .context("Failed to start A2A task")?;

        Ok(task)
    }

    /// Update task progress (0.0 to 1.0)
    pub async fn update_progress(
        pool: &DbPool,
        task_id: &Uuid,
        progress: f64,
    ) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            UPDATE a2a_tasks
            SET progress = $2::DECIMAL(3,2)
            WHERE id = $1 AND status = 'working'
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(progress)
        .fetch_optional(pool)
        .await
        .context("Failed to update A2A task progress")?;

        Ok(task)
    }

    /// Complete task with result
    pub async fn complete_task(
        pool: &DbPool,
        task_id: &Uuid,
        result: &serde_json::Value,
        cost: Option<f64>,
    ) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            UPDATE a2a_tasks
            SET status = 'completed',
                progress = 1.0,
                result = $2,
                cost = $3::DECIMAL(20,8),
                completed_at = NOW()
            WHERE id = $1 AND status IN ('submitted', 'working')
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(result)
        .bind(cost)
        .fetch_optional(pool)
        .await
        .context("Failed to complete A2A task")?;

        Ok(task)
    }

    /// Fail task with error message
    pub async fn fail_task(pool: &DbPool, task_id: &Uuid, error: &str) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            UPDATE a2a_tasks
            SET status = 'failed',
                error = $2,
                completed_at = NOW()
            WHERE id = $1 AND status IN ('submitted', 'working')
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(error)
        .fetch_optional(pool)
        .await
        .context("Failed to fail A2A task")?;

        Ok(task)
    }

    /// Cancel a pending or working task
    pub async fn cancel_task(
        pool: &DbPool,
        task_id: &Uuid,
        organization_id: &str,
    ) -> Result<Option<A2aTask>> {
        let task = sqlx::query_as::<_, A2aTask>(
            r#"
            UPDATE a2a_tasks
            SET status = 'cancelled', completed_at = NOW()
            WHERE id = $1
              AND organization_id = $2
              AND status IN ('submitted', 'working')
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .context("Failed to cancel A2A task")?;

        Ok(task)
    }

    /// List tasks for an organization with pagination
    pub async fn list_by_organization(
        pool: &DbPool,
        organization_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<A2aTask>> {
        let tasks = sqlx::query_as::<_, A2aTask>(
            r#"
            SELECT * FROM a2a_tasks
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list A2A tasks")?;

        Ok(tasks)
    }

    /// Count tasks for an organization
    pub async fn count_by_organization(pool: &DbPool, organization_id: &str) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM a2a_tasks
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .context("Failed to count A2A tasks")?;

        Ok(count.0)
    }

    /// Count pending tasks for rate limiting
    pub async fn count_pending_by_organization(
        pool: &DbPool,
        organization_id: &str,
    ) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM a2a_tasks
            WHERE organization_id = $1
              AND status IN ('submitted', 'working')
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .context("Failed to count pending A2A tasks")?;

        Ok(count.0)
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would go here
    // Requires DATABASE_URL to be set
}
