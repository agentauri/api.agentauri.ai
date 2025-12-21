//! Agent follows repository for database operations

use anyhow::{Context, Result};
use shared::models::AgentFollow;
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub struct AgentFollowRepository;

impl AgentFollowRepository {
    /// Create a new agent follow with its underlying triggers (within a transaction)
    #[allow(clippy::too_many_arguments)]
    pub async fn create<'e, E>(
        executor: E,
        agent_id: i64,
        chain_id: i32,
        organization_id: &str,
        user_id: &str,
        trigger_identity_id: &str,
        trigger_reputation_id: &str,
        trigger_validation_id: &str,
    ) -> Result<AgentFollow>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let follow = sqlx::query_as::<_, AgentFollow>(
            r#"
            INSERT INTO agent_follows (
                id, agent_id, chain_id, organization_id, user_id,
                trigger_identity_id, trigger_reputation_id, trigger_validation_id,
                enabled, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9, $9)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(agent_id)
        .bind(chain_id)
        .bind(organization_id)
        .bind(user_id)
        .bind(trigger_identity_id)
        .bind(trigger_reputation_id)
        .bind(trigger_validation_id)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to create agent follow")?;

        Ok(follow)
    }

    /// Find follow by agent_id, chain_id, and organization_id
    pub async fn find_by_agent_and_org(
        pool: &DbPool,
        agent_id: i64,
        chain_id: i32,
        organization_id: &str,
    ) -> Result<Option<AgentFollow>> {
        let follow = sqlx::query_as::<_, AgentFollow>(
            r#"
            SELECT * FROM agent_follows
            WHERE agent_id = $1 AND chain_id = $2 AND organization_id = $3
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find agent follow")?;

        Ok(follow)
    }

    /// Find follow by ID
    pub async fn find_by_id(pool: &DbPool, id: &str) -> Result<Option<AgentFollow>> {
        let follow = sqlx::query_as::<_, AgentFollow>(
            r#"
            SELECT * FROM agent_follows
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to find agent follow by ID")?;

        Ok(follow)
    }

    /// List follows for an organization with optional filters
    pub async fn list_by_organization(
        pool: &DbPool,
        organization_id: &str,
        chain_id: Option<i32>,
        enabled: Option<bool>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentFollow>> {
        let follows = sqlx::query_as::<_, AgentFollow>(
            r#"
            SELECT * FROM agent_follows
            WHERE organization_id = $1
              AND ($2::INTEGER IS NULL OR chain_id = $2)
              AND ($3::BOOLEAN IS NULL OR enabled = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(organization_id)
        .bind(chain_id)
        .bind(enabled)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list agent follows")?;

        Ok(follows)
    }

    /// Count follows for an organization
    pub async fn count_by_organization(
        pool: &DbPool,
        organization_id: &str,
        chain_id: Option<i32>,
        enabled: Option<bool>,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM agent_follows
            WHERE organization_id = $1
              AND ($2::INTEGER IS NULL OR chain_id = $2)
              AND ($3::BOOLEAN IS NULL OR enabled = $3)
            "#,
        )
        .bind(organization_id)
        .bind(chain_id)
        .bind(enabled)
        .fetch_one(pool)
        .await
        .context("Failed to count agent follows")?;

        Ok(count)
    }

    /// Update follow enabled status
    pub async fn update_enabled(pool: &DbPool, id: &str, enabled: bool) -> Result<AgentFollow> {
        let now = chrono::Utc::now();

        let follow = sqlx::query_as::<_, AgentFollow>(
            r#"
            UPDATE agent_follows
            SET enabled = $1, updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(enabled)
        .bind(now)
        .bind(id)
        .fetch_one(pool)
        .await
        .context("Failed to update agent follow")?;

        Ok(follow)
    }

    /// Delete a follow by ID (within a transaction)
    pub async fn delete<'e, E>(executor: E, id: &str) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM agent_follows
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await
        .context("Failed to delete agent follow")?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete by pool (convenience method)
    pub async fn delete_by_pool(pool: &DbPool, id: &str) -> Result<bool> {
        Self::delete(pool, id).await
    }

    /// Get trigger IDs for a follow
    pub async fn get_trigger_ids(
        pool: &DbPool,
        agent_id: i64,
        chain_id: i32,
        organization_id: &str,
    ) -> Result<Option<(String, String, String)>> {
        let result = sqlx::query_as::<_, (String, String, String)>(
            r#"
            SELECT trigger_identity_id, trigger_reputation_id, trigger_validation_id
            FROM agent_follows
            WHERE agent_id = $1 AND chain_id = $2 AND organization_id = $3
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get trigger IDs")?;

        Ok(result)
    }

    /// Check if follow belongs to organization
    pub async fn belongs_to_organization(
        pool: &DbPool,
        agent_id: i64,
        chain_id: i32,
        organization_id: &str,
    ) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM agent_follows
                WHERE agent_id = $1 AND chain_id = $2 AND organization_id = $3
            )
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .context("Failed to check follow organization")?;

        Ok(result)
    }
}
