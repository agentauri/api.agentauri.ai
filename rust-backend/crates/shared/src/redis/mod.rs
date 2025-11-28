//! Redis client and utilities
//!
//! This module provides Redis connection management and utilities for:
//! - Connection pooling
//! - Rate limiting operations
//! - Job queue management
//! - Caching

pub mod rate_limiter;

pub use rate_limiter::{RateLimitResult, RateLimitScope, RateLimiter};

use crate::error::{Error, Result};
use redis::{aio::ConnectionManager, Client};

/// Create a Redis client from configuration
pub async fn create_client(url: &str) -> Result<ConnectionManager> {
    let client = Client::open(url).map_err(|e| Error::config(format!("Invalid Redis URL: {}", e)))?;

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
