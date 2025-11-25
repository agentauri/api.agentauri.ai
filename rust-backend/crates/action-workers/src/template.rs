//! Template rendering for action messages
//!
//! Supports variable substitution using {{variable}} syntax.
//!
//! # Security
//!
//! - Variable names are restricted to a whitelist to prevent template injection
//! - Variable values are sanitized before logging to prevent log injection
//! - Message length is validated to prevent resource exhaustion

use lazy_static::lazy_static;
use regex::Regex;

use crate::error::WorkerError;

lazy_static! {
    /// Pattern for matching template variables: {{variable_name}}
    static ref VAR_PATTERN: Regex = Regex::new(r"\{\{(\w+)\}\}").expect("Invalid regex pattern");
}

/// Maximum message length for templates (Telegram limit is 4096)
pub const MAX_MESSAGE_LENGTH: usize = 4096;

/// Maximum length for variable values when logging
const MAX_VARIABLE_LOG_LENGTH: usize = 100;

/// Whitelist of allowed variable names for security
///
/// This prevents template injection attacks by only allowing known-safe variables.
const ALLOWED_VARIABLES: &[&str] = &[
    // Event identifiers
    "event_id",
    "event_type",
    "chain_id",
    "block_number",
    "transaction_hash",
    "log_index",
    "timestamp",
    // Agent data
    "agent_id",
    "owner",
    "token_uri",
    // Reputation data
    "score",
    "client_address",
    "feedback_index",
    "tag1",
    "tag2",
    "tags",
    "file_uri",
    "file_hash",
    "responder",
    "response_uri",
    // Validation data
    "validator_address",
    "request_uri",
    "request_hash",
    "response",
    "response_hash",
    "validation_tag",
    // Registry type
    "registry",
];

/// Sanitize a variable value for safe logging
///
/// # Security
///
/// Prevents log injection by:
/// - Removing control characters (newlines, tabs, etc.)
/// - Truncating excessively long values
/// - Preserving normal spaces for readability
fn sanitize_variable_for_logging(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .take(MAX_VARIABLE_LOG_LENGTH)
        .collect();

    if value.len() > MAX_VARIABLE_LOG_LENGTH {
        format!("{}...", sanitized)
    } else {
        sanitized
    }
}

/// Check if a variable name is allowed
///
/// # Security
///
/// Only whitelisted variable names are allowed to prevent template injection.
fn is_variable_allowed(var_name: &str) -> bool {
    ALLOWED_VARIABLES.contains(&var_name)
}

/// Validate template against variable whitelist
///
/// # Security
///
/// Ensures all variables in the template are in the whitelist.
/// This prevents users from injecting arbitrary variable names.
pub fn validate_template_variables(template: &str) -> Result<(), WorkerError> {
    let variables = extract_variables(template);
    let disallowed: Vec<_> = variables
        .iter()
        .filter(|name| !is_variable_allowed(name))
        .collect();

    if !disallowed.is_empty() {
        return Err(WorkerError::template(format!(
            "Template contains disallowed variables: {}. Allowed variables: {}",
            disallowed
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            ALLOWED_VARIABLES.join(", ")
        )));
    }

    Ok(())
}

/// Validate template length
///
/// # Security
///
/// Prevents resource exhaustion from excessively long templates.
pub fn validate_template_length(template: &str) -> Result<(), WorkerError> {
    if template.len() > MAX_MESSAGE_LENGTH {
        return Err(WorkerError::template(format!(
            "Template too long: {} characters (max: {})",
            template.len(),
            MAX_MESSAGE_LENGTH
        )));
    }
    Ok(())
}

/// Render a template with variable substitution
///
/// Variables in the template are specified using `{{variable_name}}` syntax.
/// Values are looked up from the provided JSON object.
///
/// # Security
///
/// - Only whitelisted variable names are allowed
/// - Variable values are sanitized before logging
/// - Template and result length are validated
///
/// # Arguments
///
/// * `template` - Template string with {{variable}} placeholders
/// * `variables` - JSON object containing variable values
///
/// # Returns
///
/// Rendered string with variables substituted
///
/// # Examples
///
/// ```ignore
/// let template = "Agent {{agent_id}} received score: {{score}}";
/// let vars = json!({"agent_id": 42, "score": 85});
/// let result = render_template(template, &vars)?;
/// // result: "Agent 42 received score: 85"
/// ```
pub fn render_template(
    template: &str,
    variables: &serde_json::Value,
) -> Result<String, WorkerError> {
    // Validate template before rendering
    validate_template_length(template)?;
    validate_template_variables(template)?;

    let mut result = template.to_string();

    // Find all variable references in template
    for cap in VAR_PATTERN.captures_iter(template) {
        let full_match = &cap[0]; // e.g., "{{agent_id}}"
        let var_name = &cap[1]; // e.g., "agent_id"

        // Look up variable value (already validated to be in whitelist)
        let value = get_variable_value(variables, var_name);

        // Replace all occurrences of this variable
        result = result.replace(full_match, &value);
    }

    // Validate result length
    if result.len() > MAX_MESSAGE_LENGTH {
        return Err(WorkerError::template(format!(
            "Rendered message too long: {} characters (max: {})",
            result.len(),
            MAX_MESSAGE_LENGTH
        )));
    }

    Ok(result)
}

/// Get a variable value from JSON, converting to string representation
///
/// # Security
///
/// Variable names are already validated against whitelist before this function is called.
fn get_variable_value(variables: &serde_json::Value, name: &str) -> String {
    match variables.get(name) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) => "null".to_string(),
        Some(serde_json::Value::Array(arr)) => {
            // Format arrays as comma-separated values
            arr.iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        Some(serde_json::Value::Object(_)) => {
            // Keep objects as JSON strings (truncate if needed)
            let json_str = variables.get(name).unwrap().to_string();
            if json_str.len() > 1000 {
                format!("{}...", &json_str[..997])
            } else {
                json_str
            }
        }
        None => {
            // Keep original placeholder if variable not found
            // Use sanitized variable name for logging (already in whitelist)
            tracing::debug!(
                variable = sanitize_variable_for_logging(name),
                "Template variable not found in data, keeping placeholder"
            );
            format!("{{{{{}}}}}", name)
        }
    }
}

/// Extract all variable names from a template
pub fn extract_variables(template: &str) -> Vec<String> {
    VAR_PATTERN
        .captures_iter(template)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Validate that all required variables are present in the data
#[cfg(test)]
pub fn validate_variables(
    template: &str,
    variables: &serde_json::Value,
) -> Result<(), WorkerError> {
    let required = extract_variables(template);
    let missing: Vec<_> = required
        .iter()
        .filter(|name| variables.get(name.as_str()).is_none())
        .collect();

    if !missing.is_empty() {
        return Err(WorkerError::template(format!(
            "Missing template variables: {}",
            missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_simple_template() {
        let template = "Agent {{agent_id}} received score: {{score}}";
        let vars = json!({"agent_id": "42", "score": 85});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Agent 42 received score: 85");
    }

    #[test]
    fn test_render_with_numbers() {
        let template = "Block {{block_number}} on chain {{chain_id}}";
        let vars = json!({"block_number": 1000000, "chain_id": 84532});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Block 1000000 on chain 84532");
    }

    #[test]
    fn test_render_with_booleans() {
        // Use a whitelisted variable
        let template = "Score: {{score}}";
        let vars = json!({"score": true});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Score: true");
    }

    #[test]
    fn test_render_missing_variable_kept() {
        // Use whitelisted variables
        let template = "Hello {{agent_id}}, your chain is {{chain_id}}";
        let vars = json!({"agent_id": "42"});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Hello 42, your chain is {{chain_id}}");
    }

    #[test]
    fn test_render_repeated_variable() {
        // Use a whitelisted variable
        let template = "{{agent_id}} is {{agent_id}} is {{agent_id}}";
        let vars = json!({"agent_id": "Bob"});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Bob is Bob is Bob");
    }

    #[test]
    fn test_render_no_variables() {
        let template = "Static message with no variables";
        let vars = json!({});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Static message with no variables");
    }

    #[test]
    fn test_render_empty_template() {
        let template = "";
        let vars = json!({"unused": "value"});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_array_variable() {
        let template = "Tags: {{tags}}";
        let vars = json!({"tags": ["trade", "reliable"]});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Tags: trade, reliable");
    }

    #[test]
    fn test_render_null_variable() {
        // Use a whitelisted variable
        let template = "Score: {{score}}";
        let vars = json!({"score": null});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Score: null");
    }

    #[test]
    fn test_extract_variables() {
        let template = "{{a}} and {{b}} and {{a}} again";
        let vars = extract_variables(template);

        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"a".to_string()));
        assert!(vars.contains(&"b".to_string()));
    }

    #[test]
    fn test_validate_variables_success() {
        // Use whitelisted variables
        let template = "{{agent_id}} {{score}}";
        let vars = json!({"agent_id": "Alice", "score": 30});

        assert!(validate_variables(template, &vars).is_ok());
    }

    #[test]
    fn test_validate_variables_missing() {
        // Use whitelisted variables
        let template = "{{agent_id}} {{score}} {{chain_id}}";
        let vars = json!({"agent_id": "Alice"});

        let result = validate_variables(template, &vars);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("score"));
        assert!(err_msg.contains("chain_id"));
    }

    #[test]
    fn test_special_characters_in_template() {
        let template = "Score: {{score}}% (threshold: 60%)";
        let vars = json!({"score": 85});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Score: 85% (threshold: 60%)");
    }

    #[test]
    fn test_multiline_template() {
        let template = "Event: {{event_type}}\nAgent: {{agent_id}}\nScore: {{score}}";
        let vars = json!({
            "event_type": "NewFeedback",
            "agent_id": 42,
            "score": 85
        });

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "Event: NewFeedback\nAgent: 42\nScore: 85");
    }

    #[test]
    fn test_markdown_template() {
        let template = "**Agent {{agent_id}}**\n\n_Score: {{score}}_";
        let vars = json!({"agent_id": 42, "score": 85});

        let result = render_template(template, &vars).unwrap();
        assert_eq!(result, "**Agent 42**\n\n_Score: 85_");
    }

    #[test]
    fn test_validate_template_variables_allowed() {
        let template = "Agent {{agent_id}} score {{score}}";
        assert!(validate_template_variables(template).is_ok());
    }

    #[test]
    fn test_validate_template_variables_disallowed() {
        let template = "Secret: {{password}} and {{api_key}}";
        let result = validate_template_variables(template);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("password"));
        assert!(err_msg.contains("api_key"));
    }

    #[test]
    fn test_validate_template_length_ok() {
        let template = "a".repeat(100);
        assert!(validate_template_length(&template).is_ok());
    }

    #[test]
    fn test_validate_template_length_too_long() {
        let template = "a".repeat(5000);
        assert!(validate_template_length(&template).is_err());
    }

    #[test]
    fn test_render_template_with_disallowed_variable() {
        let template = "Password: {{password}}";
        let vars = json!({"password": "secret123"});
        let result = render_template(template, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template_result_too_long() {
        // Create a template that expands to > MAX_MESSAGE_LENGTH
        let long_value = "x".repeat(5000);
        let template = "Data: {{agent_id}}";
        let vars = json!({"agent_id": long_value});

        // This should fail validation during rendering
        let result = render_template(template, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_variable_for_logging() {
        let value = "test\nwith\nnewlines";
        let sanitized = sanitize_variable_for_logging(value);
        assert!(!sanitized.contains('\n'));
    }

    #[test]
    fn test_sanitize_variable_for_logging_long() {
        let long = "a".repeat(200);
        let sanitized = sanitize_variable_for_logging(&long);
        assert!(sanitized.len() <= 103); // 100 + "..."
    }

    #[test]
    fn test_is_variable_allowed() {
        assert!(is_variable_allowed("agent_id"));
        assert!(is_variable_allowed("score"));
        assert!(is_variable_allowed("event_type"));
        assert!(!is_variable_allowed("password"));
        assert!(!is_variable_allowed("secret"));
        assert!(!is_variable_allowed("api_key"));
    }

    #[test]
    fn test_all_whitelisted_variables_work() {
        // Test that all whitelisted variables can be used
        for var_name in ALLOWED_VARIABLES {
            let template = format!("Value: {{{{{}}}}}", var_name);
            assert!(
                validate_template_variables(&template).is_ok(),
                "Variable {} should be allowed",
                var_name
            );
        }
    }
}
