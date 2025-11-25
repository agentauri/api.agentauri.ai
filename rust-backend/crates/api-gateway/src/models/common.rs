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

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ErrorResponse tests
    // ========================================================================

    #[test]
    fn test_error_response_new() {
        let err = ErrorResponse::new("not_found", "Resource not found");
        assert_eq!(err.error, "not_found");
        assert_eq!(err.message, "Resource not found");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_error_response_with_details() {
        let details = serde_json::json!({"field": "email", "reason": "invalid format"});
        let err = ErrorResponse::with_details("validation_error", "Validation failed", details);
        assert_eq!(err.error, "validation_error");
        assert!(err.details.is_some());
    }

    #[test]
    fn test_error_response_serialization() {
        let err = ErrorResponse::new("unauthorized", "Missing token");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("unauthorized"));
        assert!(json.contains("Missing token"));
        // details should not be in output since it's None
        assert!(!json.contains("details"));
    }

    #[test]
    fn test_error_response_serialization_with_details() {
        let err = ErrorResponse::with_details(
            "error",
            "message",
            serde_json::json!({"key": "value"}),
        );
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("details"));
        assert!(json.contains("key"));
    }

    // ========================================================================
    // SuccessResponse tests
    // ========================================================================

    #[test]
    fn test_success_response_new() {
        let resp = SuccessResponse::new("data");
        assert_eq!(resp.data, "data");
    }

    #[test]
    fn test_success_response_with_struct() {
        #[derive(Serialize)]
        struct Item {
            id: i32,
            name: String,
        }
        let item = Item {
            id: 1,
            name: "test".to_string(),
        };
        let resp = SuccessResponse::new(item);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"name\":\"test\""));
    }

    // ========================================================================
    // PaginationParams tests
    // ========================================================================

    #[test]
    fn test_pagination_params_validate_valid() {
        let params = PaginationParams {
            limit: 20,
            offset: 0,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_pagination_params_validate_max_limit() {
        let params = PaginationParams {
            limit: 100,
            offset: 0,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_pagination_params_validate_min_limit() {
        let params = PaginationParams {
            limit: 1,
            offset: 0,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_pagination_params_validate_limit_too_low() {
        let params = PaginationParams {
            limit: 0,
            offset: 0,
        };
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Limit"));
    }

    #[test]
    fn test_pagination_params_validate_limit_too_high() {
        let params = PaginationParams {
            limit: 101,
            offset: 0,
        };
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Limit"));
    }

    #[test]
    fn test_pagination_params_validate_negative_offset() {
        let params = PaginationParams {
            limit: 20,
            offset: -1,
        };
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Offset"));
    }

    #[test]
    fn test_pagination_params_default_deserialization() {
        let json = "{}";
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit, 20); // default
        assert_eq!(params.offset, 0); // default
    }

    // ========================================================================
    // PaginationMeta tests
    // ========================================================================

    #[test]
    fn test_pagination_meta_new_has_more() {
        let meta = PaginationMeta::new(100, 20, 0);
        assert_eq!(meta.total, 100);
        assert_eq!(meta.limit, 20);
        assert_eq!(meta.offset, 0);
        assert!(meta.has_more); // 0 + 20 < 100
    }

    #[test]
    fn test_pagination_meta_new_no_more() {
        let meta = PaginationMeta::new(100, 20, 80);
        assert!(!meta.has_more); // 80 + 20 >= 100
    }

    #[test]
    fn test_pagination_meta_new_exact_end() {
        let meta = PaginationMeta::new(100, 20, 80);
        assert!(!meta.has_more); // 80 + 20 = 100, no more
    }

    #[test]
    fn test_pagination_meta_new_past_end() {
        let meta = PaginationMeta::new(100, 20, 90);
        assert!(!meta.has_more); // 90 + 20 > 100
    }

    #[test]
    fn test_pagination_meta_serialization() {
        let meta = PaginationMeta::new(50, 10, 20);
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"total\":50"));
        assert!(json.contains("\"limit\":10"));
        assert!(json.contains("\"offset\":20"));
        assert!(json.contains("\"has_more\":true"));
    }

    // ========================================================================
    // PaginatedResponse tests
    // ========================================================================

    #[test]
    fn test_paginated_response_serialization() {
        let resp = PaginatedResponse {
            data: vec!["item1", "item2"],
            pagination: PaginationMeta::new(10, 2, 0),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("item1"));
        assert!(json.contains("item2"));
        assert!(json.contains("pagination"));
        assert!(json.contains("total"));
    }
}
