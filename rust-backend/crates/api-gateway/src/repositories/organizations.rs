//! Organization and Member Database Repositories
//!
//! This module provides database access for organizations and their members.
//!
//! # Repositories
//!
//! - [`OrganizationRepository`] - CRUD operations for organizations
//! - [`MemberRepository`] - Member management (add, remove, update roles)
//!
//! # JOIN Result Types
//!
//! To avoid N+1 queries, this module provides optimized structs for common patterns:
//!
//! - [`OrganizationWithRole`] - Organization data with the user's role in one query
//! - [`MemberWithUser`] - Member data with user info (username, email) in one query
//!
//! # Transaction Support
//!
//! Methods ending in `_with_executor` accept any SQLx executor, allowing them
//! to be used within transactions:
//!
//! ```ignore
//! let mut tx = pool.begin().await?;
//! OrganizationRepository::create_with_executor(&mut *tx, ...).await?;
//! MemberRepository::add_with_executor(&mut *tx, ...).await?;
//! tx.commit().await?;
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::models::{Organization, OrganizationMember};
use shared::DbPool;
use sqlx::{Executor, FromRow, Postgres};
use uuid::Uuid;

// ============================================================================
// JOIN Result Types (to avoid N+1 queries)
// ============================================================================

/// Organization with the user's role (from JOIN query)
#[derive(Debug, Clone, FromRow)]
pub struct OrganizationWithRole {
    // Organization fields
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub plan: String,
    pub is_personal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Role from membership
    pub my_role: String,
}

/// Member with user info (from JOIN query)
#[derive(Debug, Clone, FromRow)]
pub struct MemberWithUser {
    // Member fields
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    // User fields from JOIN
    pub username: String,
    pub email: String,
}

// ============================================================================
// OrganizationRepository
// ============================================================================

pub struct OrganizationRepository;

impl OrganizationRepository {
    /// Create a new organization
    pub async fn create(
        pool: &DbPool,
        name: &str,
        slug: &str,
        description: Option<&str>,
        owner_id: &str,
        is_personal: bool,
    ) -> Result<Organization> {
        Self::create_with_executor(pool, name, slug, description, owner_id, is_personal).await
    }

    /// Create a new organization with a generic executor (supports transactions)
    pub async fn create_with_executor<'e, E>(
        executor: E,
        name: &str,
        slug: &str,
        description: Option<&str>,
        owner_id: &str,
        is_personal: bool,
    ) -> Result<Organization>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (id, name, slug, description, owner_id, plan, is_personal, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'free', $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(&org_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(owner_id)
        .bind(is_personal)
        .bind(now)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to create organization")?;

        Ok(org)
    }

    /// Find organization by ID
    pub async fn find_by_id(pool: &DbPool, org_id: &str) -> Result<Option<Organization>> {
        let org = sqlx::query_as::<_, Organization>(r#"SELECT * FROM organizations WHERE id = $1"#)
            .bind(org_id)
            .fetch_optional(pool)
            .await
            .context("Failed to find organization by ID")?;

        Ok(org)
    }

    /// Find organization by slug
    #[allow(dead_code)]
    pub async fn find_by_slug(pool: &DbPool, slug: &str) -> Result<Option<Organization>> {
        let org =
            sqlx::query_as::<_, Organization>(r#"SELECT * FROM organizations WHERE slug = $1"#)
                .bind(slug)
                .fetch_optional(pool)
                .await
                .context("Failed to find organization by slug")?;

        Ok(org)
    }

    /// Check if slug already exists
    #[allow(dead_code)]
    pub async fn slug_exists(pool: &DbPool, slug: &str) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(SELECT 1 FROM organizations WHERE slug = $1)"#,
        )
        .bind(slug)
        .fetch_one(pool)
        .await
        .context("Failed to check slug existence")?;

        Ok(exists)
    }

    /// List organizations for a user (via membership) with pagination
    #[allow(dead_code)]
    pub async fn list_by_user(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT o.*
            FROM organizations o
            INNER JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list organizations")?;

        Ok(orgs)
    }

    /// List organizations for a user WITH their role (optimized, no N+1)
    pub async fn list_by_user_with_roles(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrganizationWithRole>> {
        let orgs = sqlx::query_as::<_, OrganizationWithRole>(
            r#"
            SELECT
                o.id, o.name, o.slug, o.description, o.owner_id,
                o.plan, o.is_personal, o.created_at, o.updated_at,
                om.role as my_role
            FROM organizations o
            INNER JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list organizations with roles")?;

        Ok(orgs)
    }

    /// Count organizations for a user
    pub async fn count_by_user(pool: &DbPool, user_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(DISTINCT o.id)
            FROM organizations o
            INNER JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to count organizations")?;

        Ok(count)
    }

    /// Find user's personal organization
    #[allow(dead_code)]
    pub async fn find_personal_by_user(
        pool: &DbPool,
        user_id: &str,
    ) -> Result<Option<Organization>> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE owner_id = $1 AND is_personal = true
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find personal organization")?;

        Ok(org)
    }

    /// Update organization
    ///
    /// Uses a safe COALESCE pattern instead of dynamic SQL to prevent potential issues
    /// and make the query easier to audit.
    ///
    /// - `name`: `None` = keep existing, `Some(value)` = update to value
    /// - `description`: `None` = keep existing, `Some(None)` = set to NULL, `Some(Some(value))` = update to value
    pub async fn update(
        pool: &DbPool,
        org_id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<Organization> {
        let now = chrono::Utc::now();

        // Use a static query with COALESCE/CASE patterns for safe updates
        // $1 = updated_at (always set)
        // $2 = name (NULL means keep existing)
        // $3 = should_update_description (boolean flag)
        // $4 = description (new value if updating, can be NULL)
        // $5 = org_id
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations SET
                updated_at = $1,
                name = COALESCE($2, name),
                description = CASE
                    WHEN $3 THEN $4
                    ELSE description
                END
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(now)
        .bind(name)
        .bind(description.is_some()) // should_update_description flag
        .bind(description.flatten()) // actual description value (flattened from Option<Option<&str>> to Option<&str>)
        .bind(org_id)
        .fetch_one(pool)
        .await
        .context("Failed to update organization")?;

        Ok(org)
    }

    /// Delete organization (only non-personal organizations)
    pub async fn delete(pool: &DbPool, org_id: &str) -> Result<bool> {
        let result =
            sqlx::query(r#"DELETE FROM organizations WHERE id = $1 AND is_personal = false"#)
                .bind(org_id)
                .execute(pool)
                .await
                .context("Failed to delete organization")?;

        Ok(result.rows_affected() > 0)
    }

    /// Transfer organization ownership to another member
    ///
    /// This operation:
    /// 1. Updates the organization's owner_id
    /// 2. Changes the old owner's role to 'admin'
    /// 3. Changes the new owner's role to 'owner'
    ///
    /// Returns the updated organization. Fails if the organization is personal
    /// or if the new owner is not a member.
    pub async fn transfer_ownership(
        pool: &DbPool,
        org_id: &str,
        current_owner_id: &str,
        new_owner_id: &str,
    ) -> Result<Organization> {
        // Use a transaction for atomicity
        let mut tx = pool.begin().await.context("Failed to start transaction")?;

        // Update organization owner_id (only if not personal)
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET owner_id = $1, updated_at = NOW()
            WHERE id = $2 AND is_personal = false
            RETURNING *
            "#,
        )
        .bind(new_owner_id)
        .bind(org_id)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to update organization owner")?
        .ok_or_else(|| anyhow::anyhow!("Organization not found or is a personal organization"))?;

        // Update old owner's role to admin
        sqlx::query(
            r#"
            UPDATE organization_members
            SET role = 'admin'
            WHERE organization_id = $1 AND user_id = $2 AND role = 'owner'
            "#,
        )
        .bind(org_id)
        .bind(current_owner_id)
        .execute(&mut *tx)
        .await
        .context("Failed to update old owner's role")?;

        // Update new owner's role to owner
        let rows = sqlx::query(
            r#"
            UPDATE organization_members
            SET role = 'owner'
            WHERE organization_id = $1 AND user_id = $2
            "#,
        )
        .bind(org_id)
        .bind(new_owner_id)
        .execute(&mut *tx)
        .await
        .context("Failed to update new owner's role")?;

        // Ensure the new owner was actually a member
        if rows.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "New owner is not a member of the organization"
            ));
        }

        tx.commit()
            .await
            .context("Failed to commit ownership transfer")?;

        Ok(org)
    }
}

// ============================================================================
// MemberRepository
// ============================================================================

pub struct MemberRepository;

impl MemberRepository {
    /// Add a member to an organization
    pub async fn add(
        pool: &DbPool,
        org_id: &str,
        user_id: &str,
        role: &str,
        invited_by: Option<&str>,
    ) -> Result<OrganizationMember> {
        Self::add_with_executor(pool, org_id, user_id, role, invited_by).await
    }

    /// Add a member to an organization with a generic executor (supports transactions)
    pub async fn add_with_executor<'e, E>(
        executor: E,
        org_id: &str,
        user_id: &str,
        role: &str,
        invited_by: Option<&str>,
    ) -> Result<OrganizationMember>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let member_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            INSERT INTO organization_members (id, organization_id, user_id, role, invited_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&member_id)
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .bind(invited_by)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to add organization member")?;

        Ok(member)
    }

    /// Get a member's role in an organization
    pub async fn get_role(pool: &DbPool, org_id: &str, user_id: &str) -> Result<Option<String>> {
        let role = sqlx::query_scalar::<_, String>(
            r#"SELECT role FROM organization_members WHERE organization_id = $1 AND user_id = $2"#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get member role")?;

        Ok(role)
    }

    /// Check if user is a member of organization
    pub async fn is_member(pool: &DbPool, org_id: &str, user_id: &str) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(SELECT 1 FROM organization_members WHERE organization_id = $1 AND user_id = $2)"#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to check membership")?;

        Ok(exists)
    }

    /// List members of an organization with pagination
    #[allow(dead_code)]
    pub async fn list(
        pool: &DbPool,
        org_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrganizationMember>> {
        let members = sqlx::query_as::<_, OrganizationMember>(
            r#"
            SELECT * FROM organization_members
            WHERE organization_id = $1
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list members")?;

        Ok(members)
    }

    /// List members of an organization WITH user info (optimized, no N+1)
    pub async fn list_with_users(
        pool: &DbPool,
        org_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MemberWithUser>> {
        let members = sqlx::query_as::<_, MemberWithUser>(
            r#"
            SELECT
                om.id, om.user_id, om.role, om.created_at,
                u.username, u.email
            FROM organization_members om
            INNER JOIN users u ON om.user_id = u.id
            WHERE om.organization_id = $1
            ORDER BY om.created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list members with user info")?;

        Ok(members)
    }

    /// Count members in an organization
    pub async fn count(pool: &DbPool, org_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM organization_members WHERE organization_id = $1"#,
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
        .context("Failed to count members")?;

        Ok(count)
    }

    /// Update a member's role
    pub async fn update_role(
        pool: &DbPool,
        org_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<OrganizationMember> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            UPDATE organization_members
            SET role = $1
            WHERE organization_id = $2 AND user_id = $3
            RETURNING *
            "#,
        )
        .bind(role)
        .bind(org_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to update member role")?;

        Ok(member)
    }

    /// Remove a member from organization (cannot remove owner)
    pub async fn remove(pool: &DbPool, org_id: &str, user_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"DELETE FROM organization_members WHERE organization_id = $1 AND user_id = $2 AND role != 'owner'"#,
        )
        .bind(org_id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to remove member")?;

        Ok(result.rows_affected() > 0)
    }

    /// Find member by ID
    #[allow(dead_code)]
    pub async fn find_by_id(pool: &DbPool, member_id: &str) -> Result<Option<OrganizationMember>> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"SELECT * FROM organization_members WHERE id = $1"#,
        )
        .bind(member_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find member by ID")?;

        Ok(member)
    }

    /// Find all memberships for a user (across all organizations)
    pub async fn find_by_user(pool: &DbPool, user_id: &str) -> Result<Vec<OrganizationMember>> {
        let members = sqlx::query_as::<_, OrganizationMember>(
            r#"SELECT * FROM organization_members WHERE user_id = $1 ORDER BY created_at ASC"#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("Failed to find user memberships")?;

        Ok(members)
    }
}
