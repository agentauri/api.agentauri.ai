//! PostgreSQL NOTIFY/LISTEN implementation

use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use shared::DbPool;
use sqlx::postgres::PgListener;

/// Start listening to PostgreSQL NOTIFY events
///
/// # Arguments
///
/// * `db_pool` - Database connection pool
/// * `redis_conn` - Redis connection for job queueing
pub async fn start_listening(db_pool: DbPool, redis_conn: MultiplexedConnection) -> Result<()> {
    // Create PostgreSQL listener
    let mut listener = PgListener::connect_with(&db_pool)
        .await
        .context("Failed to create PostgreSQL listener")?;

    // Listen to the 'new_event' channel
    listener
        .listen("new_event")
        .await
        .context("Failed to listen to 'new_event' channel")?;

    tracing::info!("Listening for PostgreSQL NOTIFY events on channel 'new_event'");

    loop {
        // Wait for a notification
        match listener.recv().await {
            Ok(notification) => {
                let event_id = notification.payload().to_string();
                tracing::debug!("Received notification for event: {}", event_id);

                // Process the event in a separate task
                let db_pool = db_pool.clone();
                let redis_conn = redis_conn.clone();
                tokio::spawn(async move {
                    if let Err(e) = process_event(&event_id, db_pool, redis_conn).await {
                        tracing::error!("Error processing event {}: {}", event_id, e);
                    }
                });
            }
            Err(e) => {
                tracing::error!("Error receiving notification: {}", e);
                // Wait a bit before continuing to avoid tight loop on persistent errors
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

/// Process a single event notification
///
/// # Arguments
///
/// * `event_id` - The ID of the event to process
/// * `db_pool` - Database connection pool
/// * `redis_conn` - Redis connection for job queueing
async fn process_event(
    event_id: &str,
    db_pool: DbPool,
    _redis_conn: MultiplexedConnection,
) -> Result<()> {
    // Fetch event from database
    let event = sqlx::query_as::<_, shared::models::Event>(
        r#"
        SELECT
            id, chain_id, block_number, block_hash, transaction_hash, log_index,
            registry, event_type, agent_id, timestamp, owner, token_uri, metadata_key,
            metadata_value, client_address, feedback_index, score, tag1, tag2,
            file_uri, file_hash, validator_address, request_hash, response,
            response_uri, response_hash, tag, created_at
        FROM events
        WHERE id = $1
        "#,
    )
    .bind(event_id)
    .fetch_one(&db_pool)
    .await
    .context("Failed to fetch event from database")?;

    tracing::info!(
        "Processing event: {} (chain_id={}, registry={}, event_type={})",
        event.id,
        event.chain_id,
        event.registry,
        event.event_type
    );

    // TODO: Implement trigger matching logic
    // TODO: Enqueue jobs to Redis for matched triggers

    Ok(())
}
