//! Billing repository for credits and transactions
//!
//! **Note**: Some billing features are partially implemented but not yet integrated:
//! - Credit deduction and balance checking (Phase 4-5: Query tier billing)
//! - x402 crypto payment integration (Phase 5)
//!
//! Handles storage and retrieval of organization credits, transactions,
//! and subscription data.

use anyhow::{Context, Result};
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::billing::{Credit, CreditTransaction, PaymentNonce, Subscription};

pub struct CreditRepository;

impl CreditRepository {
    /// Initialize credits for an organization (called during org creation)
    #[allow(dead_code)] // Future feature: Called during organization creation
    pub async fn initialize<'e, E>(executor: E, organization_id: &str) -> Result<Credit>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();

        let credit = sqlx::query_as::<_, Credit>(
            r#"
            INSERT INTO credits (id, organization_id, balance, currency, created_at, updated_at)
            VALUES ($1, $2, 0, 'USDC', NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(organization_id)
        .fetch_one(executor)
        .await
        .context("Failed to initialize credits")?;

        Ok(credit)
    }

    /// Get credit balance for an organization
    pub async fn get_balance(pool: &DbPool, organization_id: &str) -> Result<Option<Credit>> {
        let credit = sqlx::query_as::<_, Credit>(
            r#"
            SELECT * FROM credits
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get credit balance")?;

        Ok(credit)
    }

    /// Add credits to an organization (returns new balance)
    pub async fn add_credits<'e, E>(executor: E, organization_id: &str, amount: i64) -> Result<i64>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Update balance
        let new_balance = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE credits
            SET balance = balance + $1, updated_at = NOW()
            WHERE organization_id = $2
            RETURNING balance
            "#,
        )
        .bind(amount)
        .bind(organization_id)
        .fetch_one(executor)
        .await
        .context("Failed to add credits")?;

        Ok(new_balance)
    }

    /// Deduct credits from an organization (returns new balance, fails if insufficient)
    ///
    /// NOTE: This method is NOT safe for concurrent requests. Use `deduct_credits_atomic`
    /// instead when handling concurrent credit deductions.
    #[allow(dead_code)] // Future feature: Query tier billing
    pub async fn deduct_credits<'e, E>(
        executor: E,
        organization_id: &str,
        amount: i64,
    ) -> Result<i64>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Use a WHERE clause to prevent going negative
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE credits
            SET balance = balance - $1, updated_at = NOW()
            WHERE organization_id = $2 AND balance >= $1
            RETURNING balance
            "#,
        )
        .bind(amount)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
        .context("Failed to deduct credits")?;

        result.ok_or_else(|| anyhow::anyhow!("Insufficient balance"))
    }

    /// Deduct credits atomically with row-level locking (prevents race conditions)
    ///
    /// This method uses SELECT ... FOR UPDATE to lock the row, preventing
    /// concurrent transactions from reading stale balance data.
    #[allow(dead_code)] // Future feature: Query tier billing
    pub async fn deduct_credits_atomic(
        pool: &DbPool,
        organization_id: &str,
        amount: i64,
    ) -> Result<i64> {
        let mut tx = pool.begin().await.context("Failed to begin transaction")?;

        // Lock the row for update - prevents concurrent reads until this transaction commits
        let current_balance = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT balance FROM credits
            WHERE organization_id = $1
            FOR UPDATE
            "#,
        )
        .bind(organization_id)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to get balance with lock")?
        .ok_or_else(|| anyhow::anyhow!("Credits not initialized for organization"))?;

        // Check sufficient balance
        if current_balance < amount {
            return Err(anyhow::anyhow!(
                "Insufficient balance: {} < {}",
                current_balance,
                amount
            ));
        }

        // Update balance
        let new_balance = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE credits
            SET balance = balance - $1, updated_at = NOW()
            WHERE organization_id = $2
            RETURNING balance
            "#,
        )
        .bind(amount)
        .bind(organization_id)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to deduct credits")?;

        tx.commit().await.context("Failed to commit transaction")?;
        Ok(new_balance)
    }

    /// Check if organization has sufficient balance
    #[allow(dead_code)] // Future feature: Query tier billing
    pub async fn has_sufficient_balance(
        pool: &DbPool,
        organization_id: &str,
        amount: i64,
    ) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM credits
                WHERE organization_id = $1 AND balance >= $2
            )
            "#,
        )
        .bind(organization_id)
        .bind(amount)
        .fetch_one(pool)
        .await
        .context("Failed to check balance")?;

        Ok(result)
    }
}

/// Parameters for creating a credit transaction
pub struct CreateTransactionParams<'a> {
    pub organization_id: &'a str,
    pub amount: i64,
    pub transaction_type: &'a str,
    pub description: Option<&'a str>,
    pub reference_id: Option<&'a str>,
    pub balance_after: i64,
    pub metadata: Option<serde_json::Value>,
}

pub struct TransactionRepository;

impl TransactionRepository {
    /// Check if a transaction with the given reference_id already exists (for idempotency)
    ///
    /// NOTE: This is a non-atomic check. For atomic idempotency in webhooks,
    /// use `create_idempotent()` instead which uses INSERT ... ON CONFLICT.
    #[allow(dead_code)] // Kept for non-critical idempotency checks
    pub async fn exists_by_reference_id(pool: &DbPool, reference_id: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM credit_transactions
                WHERE reference_id = $1
            )
            "#,
        )
        .bind(reference_id)
        .fetch_one(pool)
        .await
        .context("Failed to check transaction existence")?;

        Ok(result)
    }

    /// Atomically create a transaction with idempotency guarantee
    ///
    /// Uses INSERT ... ON CONFLICT DO NOTHING to prevent race conditions.
    /// Returns Ok(Some(transaction)) if created, Ok(None) if already exists (duplicate).
    ///
    /// SECURITY: This prevents Stripe webhook replay attacks by ensuring
    /// the same reference_id can only be processed once, even under concurrent requests.
    pub async fn create_idempotent<'e, E>(
        executor: E,
        params: CreateTransactionParams<'_>,
    ) -> Result<Option<CreditTransaction>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Use INSERT ... ON CONFLICT DO NOTHING for atomic idempotency
        // The UNIQUE index on (reference_id) WHERE reference_id IS NOT NULL AND transaction_type = 'purchase'
        // ensures this works for purchase transactions
        let tx = sqlx::query_as::<_, CreditTransaction>(
            r#"
            INSERT INTO credit_transactions (
                organization_id, amount, transaction_type,
                description, reference_id, balance_after, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (reference_id) WHERE reference_id IS NOT NULL AND transaction_type = 'purchase'
            DO NOTHING
            RETURNING *
            "#,
        )
        .bind(params.organization_id)
        .bind(params.amount)
        .bind(params.transaction_type)
        .bind(params.description)
        .bind(params.reference_id)
        .bind(params.balance_after)
        .bind(params.metadata)
        .fetch_optional(executor)
        .await
        .context("Failed to create idempotent transaction")?;

        Ok(tx)
    }

    /// Record a credit transaction
    ///
    /// NOTE: For webhook handlers, prefer `create_idempotent()` which prevents race conditions.
    #[allow(dead_code)] // Kept for internal/admin transaction creation
    pub async fn create<'e, E>(
        executor: E,
        params: CreateTransactionParams<'_>,
    ) -> Result<CreditTransaction>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let tx = sqlx::query_as::<_, CreditTransaction>(
            r#"
            INSERT INTO credit_transactions (
                organization_id, amount, transaction_type,
                description, reference_id, balance_after, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING *
            "#,
        )
        .bind(params.organization_id)
        .bind(params.amount)
        .bind(params.transaction_type)
        .bind(params.description)
        .bind(params.reference_id)
        .bind(params.balance_after)
        .bind(params.metadata)
        .fetch_one(executor)
        .await
        .context("Failed to create transaction")?;

        Ok(tx)
    }

    /// Get transactions for an organization
    pub async fn list(
        pool: &DbPool,
        organization_id: &str,
        limit: i64,
        offset: i64,
        transaction_type: Option<&str>,
    ) -> Result<Vec<CreditTransaction>> {
        let txs = if let Some(tx_type) = transaction_type {
            sqlx::query_as::<_, CreditTransaction>(
                r#"
                SELECT * FROM credit_transactions
                WHERE organization_id = $1 AND transaction_type = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(organization_id)
            .bind(tx_type)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, CreditTransaction>(
                r#"
                SELECT * FROM credit_transactions
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
        }
        .context("Failed to list transactions")?;

        Ok(txs)
    }

    /// Get transaction by ID
    #[allow(dead_code)] // Future feature: GET /api/v1/billing/transactions/:id endpoint
    pub async fn find_by_id(pool: &DbPool, id: i64) -> Result<Option<CreditTransaction>> {
        let tx = sqlx::query_as::<_, CreditTransaction>(
            r#"
            SELECT * FROM credit_transactions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to find transaction")?;

        Ok(tx)
    }
}

pub struct SubscriptionRepository;

impl SubscriptionRepository {
    /// Create or update a subscription
    pub async fn upsert<'e, E>(
        executor: E,
        organization_id: &str,
        stripe_subscription_id: &str,
        stripe_customer_id: &str,
        plan: &str,
        status: &str,
    ) -> Result<Subscription>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();

        let sub = sqlx::query_as::<_, Subscription>(
            r#"
            INSERT INTO subscriptions (
                id, organization_id, stripe_subscription_id, stripe_customer_id,
                plan, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            ON CONFLICT (organization_id) DO UPDATE SET
                stripe_subscription_id = EXCLUDED.stripe_subscription_id,
                stripe_customer_id = EXCLUDED.stripe_customer_id,
                plan = EXCLUDED.plan,
                status = EXCLUDED.status,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(organization_id)
        .bind(stripe_subscription_id)
        .bind(stripe_customer_id)
        .bind(plan)
        .bind(status)
        .fetch_one(executor)
        .await
        .context("Failed to upsert subscription")?;

        Ok(sub)
    }

    /// Get subscription for an organization
    pub async fn find_by_organization(
        pool: &DbPool,
        organization_id: &str,
    ) -> Result<Option<Subscription>> {
        let sub = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT * FROM subscriptions
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find subscription")?;

        Ok(sub)
    }

    /// Update subscription status
    pub async fn update_status<'e, E>(
        executor: E,
        stripe_subscription_id: &str,
        status: &str,
    ) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE subscriptions
            SET status = $1, updated_at = NOW()
            WHERE stripe_subscription_id = $2
            "#,
        )
        .bind(status)
        .bind(stripe_subscription_id)
        .execute(executor)
        .await
        .context("Failed to update subscription status")?;

        Ok(result.rows_affected() > 0)
    }

    /// Cancel subscription
    pub async fn cancel<'e, E>(executor: E, stripe_subscription_id: &str) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE subscriptions
            SET status = 'canceled', canceled_at = NOW(), updated_at = NOW()
            WHERE stripe_subscription_id = $1
            "#,
        )
        .bind(stripe_subscription_id)
        .execute(executor)
        .await
        .context("Failed to cancel subscription")?;

        Ok(result.rows_affected() > 0)
    }
}

/// Repository for payment nonces (x402 crypto payment integration)
#[allow(dead_code)] // Future feature: x402 crypto payment
pub struct PaymentNonceRepository;

#[allow(dead_code)] // Future feature: x402 crypto payment
impl PaymentNonceRepository {
    /// Create a payment nonce (for x402 idempotency)
    pub async fn create<'e, E>(
        executor: E,
        organization_id: &str,
        nonce: &str,
        amount: i64,
        currency: &str,
        payment_method: &str,
        expires_in_minutes: i64,
    ) -> Result<PaymentNonce>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();

        let pn = sqlx::query_as::<_, PaymentNonce>(
            r#"
            INSERT INTO payment_nonces (
                id, organization_id, nonce, amount, currency, status,
                payment_method, expires_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6, NOW() + $7 * INTERVAL '1 minute', NOW())
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(organization_id)
        .bind(nonce)
        .bind(amount)
        .bind(currency)
        .bind(payment_method)
        .bind(expires_in_minutes)
        .fetch_one(executor)
        .await
        .context("Failed to create payment nonce")?;

        Ok(pn)
    }

    /// Find payment nonce by nonce value
    pub async fn find_by_nonce(pool: &DbPool, nonce: &str) -> Result<Option<PaymentNonce>> {
        let pn = sqlx::query_as::<_, PaymentNonce>(
            r#"
            SELECT * FROM payment_nonces
            WHERE nonce = $1
            "#,
        )
        .bind(nonce)
        .fetch_optional(pool)
        .await
        .context("Failed to find payment nonce")?;

        Ok(pn)
    }

    /// Mark payment nonce as completed
    pub async fn complete<'e, E>(executor: E, nonce: &str) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE payment_nonces
            SET status = 'completed', completed_at = NOW()
            WHERE nonce = $1 AND status = 'pending' AND expires_at > NOW()
            "#,
        )
        .bind(nonce)
        .execute(executor)
        .await
        .context("Failed to complete payment nonce")?;

        Ok(result.rows_affected() > 0)
    }

    /// Mark payment nonce as failed
    pub async fn fail<'e, E>(executor: E, nonce: &str, error_message: &str) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE payment_nonces
            SET status = 'failed', error_message = $2
            WHERE nonce = $1 AND status = 'pending'
            "#,
        )
        .bind(nonce)
        .bind(error_message)
        .execute(executor)
        .await
        .context("Failed to mark payment nonce as failed")?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if nonce exists and is valid (pending, not expired)
    pub async fn is_valid(pool: &DbPool, nonce: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM payment_nonces
                WHERE nonce = $1 AND status = 'pending' AND expires_at > NOW()
            )
            "#,
        )
        .bind(nonce)
        .fetch_one(pool)
        .await
        .context("Failed to check payment nonce validity")?;

        Ok(result)
    }

    /// Cleanup expired payment nonces
    pub async fn cleanup_expired(pool: &DbPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE payment_nonces
            SET status = 'expired'
            WHERE status = 'pending' AND expires_at < NOW()
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to cleanup expired payment nonces")?;

        Ok(result.rows_affected())
    }
}
