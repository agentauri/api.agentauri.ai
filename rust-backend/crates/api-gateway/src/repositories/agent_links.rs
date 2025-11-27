//! Agent links repository for database operations
//!
//! Handles storage and retrieval of agent-to-organization links.

use anyhow::{Context, Result};
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::wallet::AgentLink;

pub struct AgentLinkRepository;

impl AgentLinkRepository {
    /// Create a new agent link
    pub async fn create<'e, E>(
        executor: E,
        agent_id: i64,
        chain_id: i32,
        organization_id: &str,
        wallet_address: &str,
        linked_by: &str,
        signature: &str,
    ) -> Result<AgentLink>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();

        let link = sqlx::query_as::<_, AgentLink>(
            r#"
            INSERT INTO agent_links (
                id, agent_id, chain_id, organization_id,
                wallet_address, linked_by, signature, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', NOW())
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(agent_id)
        .bind(chain_id)
        .bind(organization_id)
        .bind(wallet_address)
        .bind(linked_by)
        .bind(signature)
        .fetch_one(executor)
        .await
        .context("Failed to create agent link")?;

        Ok(link)
    }

    /// Find an agent link by agent_id and chain_id
    pub async fn find_by_agent(
        pool: &DbPool,
        agent_id: i64,
        chain_id: i32,
    ) -> Result<Option<AgentLink>> {
        let link = sqlx::query_as::<_, AgentLink>(
            r#"
            SELECT * FROM agent_links
            WHERE agent_id = $1 AND chain_id = $2 AND status = 'active'
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find agent link")?;

        Ok(link)
    }

    /// Find all agent links for an organization
    pub async fn find_by_organization(
        pool: &DbPool,
        organization_id: &str,
    ) -> Result<Vec<AgentLink>> {
        let links = sqlx::query_as::<_, AgentLink>(
            r#"
            SELECT * FROM agent_links
            WHERE organization_id = $1 AND status = 'active'
            ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
        .context("Failed to find agent links by organization")?;

        Ok(links)
    }

    /// Find agent link by ID
    pub async fn find_by_id(pool: &DbPool, id: &str) -> Result<Option<AgentLink>> {
        let link = sqlx::query_as::<_, AgentLink>(
            r#"
            SELECT * FROM agent_links
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to find agent link by ID")?;

        Ok(link)
    }

    /// Revoke an agent link
    pub async fn revoke<'e, E>(executor: E, agent_id: i64, chain_id: i32) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE agent_links
            SET status = 'revoked', revoked_at = NOW()
            WHERE agent_id = $1 AND chain_id = $2 AND status = 'active'
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .execute(executor)
        .await
        .context("Failed to revoke agent link")?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if an agent is already linked to any organization
    pub async fn is_agent_linked(pool: &DbPool, agent_id: i64, chain_id: i32) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM agent_links
                WHERE agent_id = $1 AND chain_id = $2 AND status = 'active'
            )
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .fetch_one(pool)
        .await
        .context("Failed to check if agent is linked")?;

        Ok(result)
    }

    /// Get the organization ID for a linked agent
    pub async fn get_organization_for_agent(
        pool: &DbPool,
        agent_id: i64,
        chain_id: i32,
    ) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, String>(
            r#"
            SELECT organization_id FROM agent_links
            WHERE agent_id = $1 AND chain_id = $2 AND status = 'active'
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get organization for agent")?;

        Ok(result)
    }
}
