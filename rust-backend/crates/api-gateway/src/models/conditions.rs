//! Trigger Condition DTOs

use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request to create a new condition
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"condition_type": "score_threshold", "field": "score", "operator": "<", "value": "60"}))]
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
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"value": "70"}))]
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

    // ========================================================================
    // JSON Deserialization tests - config field variations
    // These tests verify that the config field correctly handles all JSON states
    // ========================================================================

    #[test]
    fn test_create_condition_request_json_without_config_field() {
        // Test: JSON body does not include the config field at all
        let json = r#"{
            "condition_type": "score_threshold",
            "field": "score",
            "operator": "<",
            "value": "60"
        }"#;

        let req: Result<CreateConditionRequest, _> = serde_json::from_str(json);
        assert!(
            req.is_ok(),
            "Should deserialize when config field is omitted"
        );
        let req = req.unwrap();
        assert!(
            req.config.is_none(),
            "config should be None when field is omitted"
        );
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_condition_request_json_with_null_config() {
        // Test: JSON body has config: null explicitly
        let json = r#"{
            "condition_type": "score_threshold",
            "field": "score",
            "operator": "<",
            "value": "60",
            "config": null
        }"#;

        let req: Result<CreateConditionRequest, _> = serde_json::from_str(json);
        assert!(req.is_ok(), "Should deserialize when config is null");
        let req = req.unwrap();
        assert!(
            req.config.is_none(),
            "config should be None when explicitly null"
        );
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_condition_request_json_with_empty_object_config() {
        // Test: JSON body has config: {}
        let json = r#"{
            "condition_type": "score_threshold",
            "field": "score",
            "operator": "<",
            "value": "60",
            "config": {}
        }"#;

        let req: Result<CreateConditionRequest, _> = serde_json::from_str(json);
        assert!(
            req.is_ok(),
            "Should deserialize when config is empty object"
        );
        let req = req.unwrap();
        assert!(req.validate().is_ok());
        assert!(
            req.config.is_some(),
            "config should be Some when empty object"
        );
        assert_eq!(req.config.unwrap(), serde_json::json!({}));
    }

    #[test]
    fn test_create_condition_request_json_with_populated_config() {
        // Test: JSON body has config with actual data
        let json = r#"{
            "condition_type": "ema_threshold",
            "field": "score",
            "operator": "<",
            "value": "70",
            "config": {"window_size": 10, "alpha": 0.2}
        }"#;

        let req: Result<CreateConditionRequest, _> = serde_json::from_str(json);
        assert!(req.is_ok(), "Should deserialize when config has data");
        let req = req.unwrap();
        assert!(req.validate().is_ok());
        assert!(req.config.is_some());
        let config = req.config.unwrap();
        assert_eq!(config["window_size"], 10);
    }

    #[test]
    fn test_update_condition_request_json_without_config() {
        // Test: config field omitted - should not update config
        let json = r#"{
            "value": "70"
        }"#;

        let req: Result<UpdateConditionRequest, _> = serde_json::from_str(json);
        assert!(req.is_ok());
        let req = req.unwrap();
        assert!(req.config.is_none(), "config should be None when omitted");
    }

    #[test]
    fn test_update_condition_request_json_with_null_config() {
        // Test: config: null - serde deserializes JSON null to None
        let json = r#"{
            "value": "70",
            "config": null
        }"#;

        let req: Result<UpdateConditionRequest, _> = serde_json::from_str(json);
        assert!(req.is_ok());
        let req = req.unwrap();
        // JSON null becomes None for Option<Value>
        assert!(req.config.is_none());
    }

    #[test]
    fn test_update_condition_request_json_with_empty_config() {
        // Test: config: {} - should update config to empty object
        let json = r#"{
            "value": "70",
            "config": {}
        }"#;

        let req: Result<UpdateConditionRequest, _> = serde_json::from_str(json);
        assert!(req.is_ok());
        let req = req.unwrap();
        assert!(req.config.is_some());
        assert_eq!(req.config.unwrap(), serde_json::json!({}));
    }
}
