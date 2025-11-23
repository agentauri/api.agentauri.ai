//! Error types for the application

use thiserror::Error;

/// Result type alias using our custom Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Application error types
#[derive(Debug, Error)]
pub enum Error {
    /// Database errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found errors
    #[error("{entity} not found: {id}")]
    NotFound { entity: String, id: String },

    /// Authentication errors
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization errors
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a NotFound error
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Create a Validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a Config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an Authentication error
    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    /// Create an Authorization error
    pub fn authorization(msg: impl Into<String>) -> Self {
        Self::Authorization(msg.into())
    }

    /// Create an Internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
