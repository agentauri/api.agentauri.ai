//! Trigger Condition DTOs

use serde::Deserialize;
use validator::Validate;

/// Request to create a new condition
#[derive(Debug, Deserialize, Validate)]
pub struct CreateConditionRequest {
    #[validate(length(min = 1, max = 100))]
    pub condition_type: String,

    #[validate(length(min = 1, max = 255))]
    pub field: String,

    #[validate(length(min = 1, max = 50))]
    pub operator: String,

    #[validate(length(min = 1, max = 1000))]
    pub value: String,

    pub config: Option<serde_json::Value>,
}

/// Request to update a condition
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateConditionRequest {
    #[validate(length(min = 1, max = 100))]
    pub condition_type: Option<String>,

    #[validate(length(min = 1, max = 255))]
    pub field: Option<String>,

    #[validate(length(min = 1, max = 50))]
    pub operator: Option<String>,

    #[validate(length(min = 1, max = 1000))]
    pub value: Option<String>,

    pub config: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // CreateConditionRequest validation tests
    // ========================================================================

    #[test]
    fn test_create_condition_request_valid() {
        let req = CreateConditionRequest {
            condition_type: "score_threshold".to_string(),
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            config: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_condition_request_with_config() {
        let req = CreateConditionRequest {
            condition_type: "agent_id_equals".to_string(),
            field: "agent_id".to_string(),
            operator: "=".to_string(),
            value: "42".to_string(),
            config: Some(serde_json::json!({"extra": "data"})),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_condition_request_empty_condition_type() {
        let req = CreateConditionRequest {
            condition_type: "".to_string(),
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("condition_type"));
    }

    #[test]
    fn test_create_condition_request_condition_type_too_long() {
        let req = CreateConditionRequest {
            condition_type: "a".repeat(101), // max 100
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("condition_type"));
    }

    #[test]
    fn test_create_condition_request_empty_field() {
        let req = CreateConditionRequest {
            condition_type: "score_threshold".to_string(),
            field: "".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("field"));
    }

    #[test]
    fn test_create_condition_request_empty_operator() {
        let req = CreateConditionRequest {
            condition_type: "score_threshold".to_string(),
            field: "score".to_string(),
            operator: "".to_string(),
            value: "60".to_string(),
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("operator"));
    }

    #[test]
    fn test_create_condition_request_empty_value() {
        let req = CreateConditionRequest {
            condition_type: "score_threshold".to_string(),
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "".to_string(),
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("value"));
    }

    #[test]
    fn test_create_condition_request_value_too_long() {
        let req = CreateConditionRequest {
            condition_type: "score_threshold".to_string(),
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "a".repeat(1001), // max 1000
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("value"));
    }

    // ========================================================================
    // UpdateConditionRequest validation tests
    // ========================================================================

    #[test]
    fn test_update_condition_request_valid() {
        let req = UpdateConditionRequest {
            condition_type: Some("tag_equals".to_string()),
            field: Some("tag1".to_string()),
            operator: Some("=".to_string()),
            value: Some("trade".to_string()),
            config: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_condition_request_all_none() {
        let req = UpdateConditionRequest {
            condition_type: None,
            field: None,
            operator: None,
            value: None,
            config: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_condition_request_partial() {
        let req = UpdateConditionRequest {
            condition_type: None,
            field: None,
            operator: None,
            value: Some("70".to_string()),
            config: None,
        };
        assert!(req.validate().is_ok());
    }
}
