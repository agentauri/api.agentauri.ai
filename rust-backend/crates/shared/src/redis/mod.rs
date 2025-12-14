//! Redis client and utilities
//!
//! This module provides Redis connection management and utilities for:
//! - Connection pooling
//! - Rate limiting operations
//! - Job queue management
//! - Entity caching (users, organizations, triggers)

pub mod cache;
pub mod rate_limiter;

pub use cache::{
    get_or_fetch, membership_key, org_key_by_id, org_keys_pattern, trigger_key_by_id,
    user_key_by_email, user_key_by_id, user_key_by_username, user_keys_pattern, CacheAware,
    EntityCache,
};
pub use rate_limiter::{RateLimitResult, RateLimitScope, RateLimiter};

use crate::error::{Error, Result};
use redis::{aio::ConnectionManager, Client};

/// Create a Redis client from configuration
pub async fn create_client(url: &str) -> Result<ConnectionManager> {
    let client =
        Client::open(url).map_err(|e| Error::config(format!("Invalid Redis URL: {}", e)))?;

    ConnectionManager::new(client)
        .await
        .map_err(|e| Error::internal(format!("Failed to connect to Redis: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_redis_url() {
        let result = create_client("invalid://url").await;
        assert!(result.is_err());
    }
}
