//! Configuration management using environment variables

use crate::error::{Error, Result};
use serde::Deserialize;
use std::env;

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Database configuration
    pub database: DatabaseConfig,

    /// Redis configuration
    pub redis: RedisConfig,

    /// Server configuration
    pub server: ServerConfig,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Database host
    pub host: String,

    /// Database port
    pub port: u16,

    /// Database name
    pub name: String,

    /// Database user
    pub user: String,

    /// Database password
    pub password: String,

    /// Maximum number of connections in the pool
    pub max_connections: u32,
}

impl DatabaseConfig {
    /// Build a PostgreSQL connection URL
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.name
        )
    }
}

/// Redis configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    /// Redis host
    pub host: String,

    /// Redis port
    pub port: u16,

    /// Redis password (optional)
    pub password: Option<String>,
}

impl RedisConfig {
    /// Build a Redis connection URL
    pub fn connection_url(&self) -> String {
        if let Some(password) = &self.password {
            format!("redis://:{}@{}:{}", password, self.host, self.port)
        } else {
            format!("redis://{}:{}", self.host, self.port)
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,

    /// Server port
    pub port: u16,

    /// JWT secret for authentication
    pub jwt_secret: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        Ok(Self {
            database: DatabaseConfig {
                host: env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: env::var("DB_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_PORT: {}", e)))?,
                name: env::var("DB_NAME").unwrap_or_else(|_| "erc8004_backend".to_string()),
                user: env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: env::var("DB_PASSWORD")
                    .map_err(|_| Error::config("DB_PASSWORD must be set"))?,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_MAX_CONNECTIONS: {}", e)))?,
            },
            redis: RedisConfig {
                host: env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: env::var("REDIS_PORT")
                    .unwrap_or_else(|_| "6379".to_string())
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid REDIS_PORT: {}", e)))?,
                password: env::var("REDIS_PASSWORD").ok(),
            },
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid SERVER_PORT: {}", e)))?,
                jwt_secret: if cfg!(debug_assertions) {
                    // Development mode: Allow default
                    env::var("JWT_SECRET").unwrap_or_else(|_| {
                        tracing::warn!("Using development JWT secret. DO NOT use in production!");
                        "dev_secret_change_in_production".to_string()
                    })
                } else {
                    // Production mode: JWT_SECRET is required
                    env::var("JWT_SECRET")
                        .expect("JWT_SECRET environment variable must be set in production")
                },
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_connection_url() {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            name: "testdb".to_string(),
            user: "testuser".to_string(),
            password: "testpass".to_string(),
            max_connections: 10,
        };

        assert_eq!(
            config.connection_url(),
            "postgres://testuser:testpass@localhost:5432/testdb"
        );
    }

    #[test]
    fn test_redis_connection_url_with_password() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: Some("secret".to_string()),
        };

        assert_eq!(config.connection_url(), "redis://:secret@localhost:6379");
    }

    #[test]
    fn test_redis_connection_url_without_password() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: None,
        };

        assert_eq!(config.connection_url(), "redis://localhost:6379");
    }
}
