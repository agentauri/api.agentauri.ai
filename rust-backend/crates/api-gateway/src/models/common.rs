//! Common DTOs shared across multiple resources

use serde::{Deserialize, Serialize};

/// Standard error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        error: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: Some(details),
        }
    }
}

/// Standard success response
#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

impl<T> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

impl PaginationParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.limit < 1 || self.limit > 100 {
            return Err("Limit must be between 1 and 100".to_string());
        }
        if self.offset < 0 {
            return Err("Offset must be non-negative".to_string());
        }
        Ok(())
    }
}

/// Paginated response
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl PaginationMeta {
    pub fn new(total: i64, limit: i64, offset: i64) -> Self {
        Self {
            total,
            limit,
            offset,
            has_more: offset + limit < total,
        }
    }
}
