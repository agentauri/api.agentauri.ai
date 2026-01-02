//! Configuration management using environment variables
//!
//! # Security
//!
//! This module enforces security requirements for sensitive configuration:
//! - JWT_SECRET must be at least 32 characters (256 bits of entropy)
//! - Production mode rejects weak or default secrets
//! - Development mode warns but allows weaker secrets for testing
//!
//! Rust guideline compliant 2025-01-28

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

    /// Minimum number of connections to keep warm (pre-warmed on startup)
    pub min_connections: u32,

    /// Connection acquire timeout in seconds (fail fast if pool exhausted)
    pub acquire_timeout_secs: u64,

    /// Idle connection timeout in seconds (recycle unused connections)
    pub idle_timeout_secs: u64,

    /// Maximum connection lifetime in seconds (prevent stale connections)
    pub max_lifetime_secs: u64,

    /// SSL mode for database connection
    /// Options: disable, allow, prefer, require, verify-ca, verify-full
    /// Default: prefer (development), verify-full (production)
    pub ssl_mode: String,

    /// Optional read replica configuration for read scaling
    /// When configured, SELECT queries should use the read pool
    pub read_replica: Option<DatabaseReadReplicaConfig>,
}

/// Read replica configuration (optional)
/// Used for scaling read-heavy workloads without affecting write performance
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseReadReplicaConfig {
    /// Read replica host
    pub host: String,

    /// Read replica port (defaults to primary port if not set)
    pub port: Option<u16>,

    /// Maximum connections for read pool (can be higher than write pool)
    pub max_connections: u32,

    /// Minimum connections for read pool
    pub min_connections: u32,
}

impl DatabaseConfig {
    /// Build a PostgreSQL connection URL with SSL mode (primary/write)
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.user, self.password, self.host, self.port, self.name, self.ssl_mode
        )
    }

    /// Build a PostgreSQL connection URL for read replica
    /// Returns None if no read replica is configured
    pub fn read_replica_url(&self) -> Option<String> {
        self.read_replica.as_ref().map(|replica| {
            let port = replica.port.unwrap_or(self.port);
            format!(
                "postgres://{}:{}@{}:{}/{}?sslmode={}",
                self.user, self.password, replica.host, port, self.name, self.ssl_mode
            )
        })
    }

    /// Check if read replica is configured
    pub fn has_read_replica(&self) -> bool {
        self.read_replica.is_some()
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

    /// Direct Redis URL (takes precedence over host/port/password)
    /// Supports both `redis://` and `rediss://` (TLS) schemes
    pub url: Option<String>,
}

impl RedisConfig {
    /// Build a Redis connection URL
    ///
    /// If `url` is set (from REDIS_URL env var), uses that directly.
    /// Otherwise, builds URL from host/port/password components.
    ///
    /// This supports:
    /// - `redis://` - standard Redis connection
    /// - `rediss://` - Redis with TLS (required for AWS ElastiCache with transit encryption)
    pub fn connection_url(&self) -> String {
        // If direct URL is provided (e.g., from AWS Secrets Manager), use it
        if let Some(url) = &self.url {
            return url.clone();
        }

        // Otherwise build from components (backward compatibility)
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
                name: env::var("DB_NAME").unwrap_or_else(|_| "agentauri_backend".to_string()),
                user: env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: env::var("DB_PASSWORD")
                    .map_err(|_| Error::config("DB_PASSWORD must be set"))?,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "50".to_string()) // Reduced from 100 for better scaling
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_MAX_CONNECTIONS: {}", e)))?,
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "5".to_string()) // Pre-warm connections
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_MIN_CONNECTIONS: {}", e)))?,
                acquire_timeout_secs: env::var("DB_ACQUIRE_TIMEOUT")
                    .unwrap_or_else(|_| "5".to_string()) // Fail fast (was 30s)
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_ACQUIRE_TIMEOUT: {}", e)))?,
                idle_timeout_secs: env::var("DB_IDLE_TIMEOUT")
                    .unwrap_or_else(|_| "180".to_string()) // 3 min (was 10 min)
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_IDLE_TIMEOUT: {}", e)))?,
                max_lifetime_secs: env::var("DB_MAX_LIFETIME")
                    .unwrap_or_else(|_| "900".to_string()) // 15 min (was 30 min)
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid DB_MAX_LIFETIME: {}", e)))?,
                ssl_mode: env::var("DB_SSL_MODE").unwrap_or_else(|_| {
                    if cfg!(debug_assertions) {
                        "prefer".to_string() // Development: prefer TLS but don't require
                    } else {
                        "verify-full".to_string() // Production: require TLS with certificate verification
                    }
                }),
                read_replica: Self::load_read_replica_config()?,
            },
            redis: RedisConfig {
                host: env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: env::var("REDIS_PORT")
                    .unwrap_or_else(|_| "6379".to_string())
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid REDIS_PORT: {}", e)))?,
                password: env::var("REDIS_PASSWORD").ok(),
                // REDIS_URL takes precedence - supports TLS (rediss://) for AWS ElastiCache
                url: env::var("REDIS_URL").ok(),
            },
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|e| Error::config(format!("Invalid SERVER_PORT: {}", e)))?,
                jwt_secret: Self::load_and_validate_jwt_secret()?,
            },
        })
    }

    /// Load and validate the JWT secret with security checks
    ///
    /// # Security Requirements
    ///
    /// - Minimum 32 characters (256 bits of entropy) in production
    /// - Cannot be a known default/example value in production
    /// - Development mode allows weaker secrets with warnings
    ///
    /// # Errors
    ///
    /// Returns an error if the secret doesn't meet security requirements in production.
    fn load_and_validate_jwt_secret() -> Result<String> {
        // Known weak/default patterns that should never be used in production
        const WEAK_PATTERNS: &[&str] = &[
            "dev_secret",
            "change_me",
            "changeme",
            "example",
            "secret",
            "password",
            "test",
            "default",
            "your_secret",
            "your-secret",
            "jwt_secret",
            "jwt-secret",
        ];

        let secret = if cfg!(debug_assertions) {
            // Development mode: Allow default but warn
            env::var("JWT_SECRET").unwrap_or_else(|_| {
                tracing::warn!(
                    "JWT_SECRET not set - using development default. \
                     DO NOT use in production!"
                );
                "dev_secret_change_in_production_32chars".to_string()
            })
        } else {
            // Production mode: JWT_SECRET is required
            env::var("JWT_SECRET").map_err(|_| {
                Error::config("JWT_SECRET environment variable must be set in production")
            })?
        };

        // Validate minimum length (256 bits = 32 bytes minimum)
        if !cfg!(debug_assertions) && secret.len() < 32 {
            return Err(Error::config(format!(
                "JWT_SECRET must be at least 32 characters (256 bits of entropy). \
                 Current length: {} characters. \
                 Generate a secure secret with: openssl rand -base64 32",
                secret.len()
            )));
        }

        // Check for weak/default patterns
        let secret_lower = secret.to_lowercase();
        for pattern in WEAK_PATTERNS {
            if secret_lower.contains(pattern) {
                if cfg!(debug_assertions) {
                    tracing::error!(
                        "JWT_SECRET contains weak pattern '{}'. \
                         This is acceptable in development but MUST be changed for production!",
                        pattern
                    );
                } else {
                    return Err(Error::config(format!(
                        "JWT_SECRET contains weak pattern '{}'. \
                         Use a cryptographically secure random value. \
                         Generate with: openssl rand -base64 32",
                        pattern
                    )));
                }
            }
        }

        // In production, also check for low entropy (all same char, sequential, etc.)
        if !cfg!(debug_assertions) {
            let unique_chars: std::collections::HashSet<char> = secret.chars().collect();
            if unique_chars.len() < 10 {
                return Err(Error::config(
                    "JWT_SECRET has low entropy (too few unique characters). \
                     Use a cryptographically secure random value. \
                     Generate with: openssl rand -base64 32"
                        .to_string(),
                ));
            }
        }

        Ok(secret)
    }

    /// Load optional read replica configuration from environment variables
    ///
    /// Environment variables:
    /// - `DB_READ_HOST`: Read replica host (required to enable read replica)
    /// - `DB_READ_PORT`: Read replica port (optional, defaults to DB_PORT)
    /// - `DB_READ_MAX_CONNECTIONS`: Max connections for read pool (optional, defaults to 100)
    /// - `DB_READ_MIN_CONNECTIONS`: Min connections for read pool (optional, defaults to 10)
    ///
    /// Returns `None` if `DB_READ_HOST` is not set.
    fn load_read_replica_config() -> Result<Option<DatabaseReadReplicaConfig>> {
        match env::var("DB_READ_HOST") {
            Ok(host) => {
                let port = env::var("DB_READ_PORT")
                    .ok()
                    .map(|p| {
                        p.parse::<u16>()
                            .map_err(|e| Error::config(format!("Invalid DB_READ_PORT: {}", e)))
                    })
                    .transpose()?;

                let max_connections = env::var("DB_READ_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "100".to_string()) // Higher default for read scaling
                    .parse()
                    .map_err(|e| {
                        Error::config(format!("Invalid DB_READ_MAX_CONNECTIONS: {}", e))
                    })?;

                let min_connections = env::var("DB_READ_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .map_err(|e| {
                        Error::config(format!("Invalid DB_READ_MIN_CONNECTIONS: {}", e))
                    })?;

                tracing::info!(
                    "Read replica configured: host={}, port={:?}, max_conn={}, min_conn={}",
                    host,
                    port,
                    max_connections,
                    min_connections
                );

                Ok(Some(DatabaseReadReplicaConfig {
                    host,
                    port,
                    max_connections,
                    min_connections,
                }))
            }
            Err(_) => Ok(None), // No read replica configured
        }
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
            min_connections: 2,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 180,
            max_lifetime_secs: 900,
            ssl_mode: "prefer".to_string(),
            read_replica: None,
        };

        assert_eq!(
            config.connection_url(),
            "postgres://testuser:testpass@localhost:5432/testdb?sslmode=prefer"
        );
    }

    #[test]
    fn test_database_connection_url_with_verify_full() {
        let config = DatabaseConfig {
            host: "db.production.example.com".to_string(),
            port: 5432,
            name: "proddb".to_string(),
            user: "appuser".to_string(),
            password: "secure_password".to_string(),
            max_connections: 50,
            min_connections: 5,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 180,
            max_lifetime_secs: 900,
            ssl_mode: "verify-full".to_string(),
            read_replica: None,
        };

        assert_eq!(
            config.connection_url(),
            "postgres://appuser:secure_password@db.production.example.com:5432/proddb?sslmode=verify-full"
        );
    }

    #[test]
    fn test_redis_connection_url_with_password() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: Some("secret".to_string()),
            url: None,
        };

        assert_eq!(config.connection_url(), "redis://:secret@localhost:6379");
    }

    #[test]
    fn test_redis_connection_url_without_password() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            url: None,
        };

        assert_eq!(config.connection_url(), "redis://localhost:6379");
    }

    #[test]
    fn test_redis_connection_url_with_direct_url() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: Some("ignored".to_string()),
            url: Some("rediss://:authtoken@redis.example.com:6379".to_string()),
        };

        // Direct URL takes precedence over host/port/password
        assert_eq!(
            config.connection_url(),
            "rediss://:authtoken@redis.example.com:6379"
        );
    }

    #[test]
    fn test_redis_connection_url_tls() {
        let config = RedisConfig {
            host: "unused".to_string(),
            port: 6379,
            password: None,
            url: Some("rediss://:mytoken@master.cache.amazonaws.com:6379".to_string()),
        };

        // Supports rediss:// scheme for TLS (AWS ElastiCache)
        assert_eq!(
            config.connection_url(),
            "rediss://:mytoken@master.cache.amazonaws.com:6379"
        );
    }

    #[test]
    fn test_read_replica_url_with_config() {
        let config = DatabaseConfig {
            host: "primary.db.example.com".to_string(),
            port: 5432,
            name: "mydb".to_string(),
            user: "appuser".to_string(),
            password: "secret".to_string(),
            max_connections: 50,
            min_connections: 5,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 180,
            max_lifetime_secs: 900,
            ssl_mode: "verify-full".to_string(),
            read_replica: Some(DatabaseReadReplicaConfig {
                host: "replica.db.example.com".to_string(),
                port: Some(5433),
                max_connections: 100,
                min_connections: 10,
            }),
        };

        assert!(config.has_read_replica());
        assert_eq!(
            config.read_replica_url().unwrap(),
            "postgres://appuser:secret@replica.db.example.com:5433/mydb?sslmode=verify-full"
        );
    }

    #[test]
    fn test_read_replica_url_default_port() {
        let config = DatabaseConfig {
            host: "primary.db.example.com".to_string(),
            port: 5432,
            name: "mydb".to_string(),
            user: "appuser".to_string(),
            password: "secret".to_string(),
            max_connections: 50,
            min_connections: 5,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 180,
            max_lifetime_secs: 900,
            ssl_mode: "prefer".to_string(),
            read_replica: Some(DatabaseReadReplicaConfig {
                host: "replica.db.example.com".to_string(),
                port: None, // Should default to primary port
                max_connections: 100,
                min_connections: 10,
            }),
        };

        assert_eq!(
            config.read_replica_url().unwrap(),
            "postgres://appuser:secret@replica.db.example.com:5432/mydb?sslmode=prefer"
        );
    }

    #[test]
    fn test_no_read_replica() {
        let config = DatabaseConfig {
            host: "primary.db.example.com".to_string(),
            port: 5432,
            name: "mydb".to_string(),
            user: "appuser".to_string(),
            password: "secret".to_string(),
            max_connections: 50,
            min_connections: 5,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 180,
            max_lifetime_secs: 900,
            ssl_mode: "prefer".to_string(),
            read_replica: None,
        };

        assert!(!config.has_read_replica());
        assert!(config.read_replica_url().is_none());
    }
}
