//! MCP Tool definitions and handlers

use crate::client::{AgentAuriClient, CreateTriggerRequest};
use crate::protocol::{Tool, ToolCallResult};
use serde::Deserialize;
use serde_json::{json, Value};

/// Get all available tools
pub fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_triggers".to_string(),
            description: "List all triggers for the authenticated user. Triggers are event-driven automation rules that execute actions when blockchain events match specified conditions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page number (default: 1)",
                        "minimum": 1
                    },
                    "per_page": {
                        "type": "integer",
                        "description": "Items per page (default: 20, max: 100)",
                        "minimum": 1,
                        "maximum": 100
                    }
                }
            }),
        },
        Tool {
            name: "get_trigger".to_string(),
            description: "Get details of a specific trigger by ID, including its conditions and actions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "trigger_id": {
                        "type": "string",
                        "description": "The UUID of the trigger to retrieve"
                    }
                },
                "required": ["trigger_id"]
            }),
        },
        Tool {
            name: "create_trigger".to_string(),
            description: "Create a new trigger to monitor blockchain events. Supported registries: identity, reputation, validation. Event types vary by registry (e.g., AgentRegistered, NewFeedback, etc.).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Human-readable name for the trigger"
                    },
                    "registry": {
                        "type": "string",
                        "enum": ["identity", "reputation", "validation"],
                        "description": "The ERC-8004 registry to monitor"
                    },
                    "event_type": {
                        "type": "string",
                        "description": "The event type to monitor (e.g., AgentRegistered, NewFeedback, ValidationSubmitted)"
                    },
                    "chain_id": {
                        "type": "string",
                        "description": "Chain ID to filter events (e.g., '8453' for Base). Use '*' or omit for all chains."
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Whether the trigger is enabled (default: true)"
                    }
                },
                "required": ["name", "registry", "event_type"]
            }),
        },
        Tool {
            name: "delete_trigger".to_string(),
            description: "Delete an existing trigger by ID.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "trigger_id": {
                        "type": "string",
                        "description": "The UUID of the trigger to delete"
                    }
                },
                "required": ["trigger_id"]
            }),
        },
        Tool {
            name: "list_linked_agents".to_string(),
            description: "List all on-chain agents linked to your account. Linked agents are agents you own and have cryptographically verified ownership of.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "list_following".to_string(),
            description: "List all agents you are following. Following an agent allows you to receive notifications about their activities without owning them.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "query_events".to_string(),
            description: "Query blockchain events from the Ponder indexer. Events include agent registrations, reputation feedback, and validation submissions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "event_type": {
                        "type": "string",
                        "description": "Filter by event type (e.g., AgentRegistered, NewFeedback)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of events to return (default: 50, max: 1000)",
                        "minimum": 1,
                        "maximum": 1000
                    }
                }
            }),
        },
        Tool {
            name: "get_indexer_status".to_string(),
            description: "Get the current status of the Ponder blockchain indexer, including sync status for each monitored chain.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "get_credits".to_string(),
            description: "Get the current credit balance for your account. Credits are used for API calls and trigger executions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "list_organizations".to_string(),
            description: "List all organizations you are a member of. Organizations allow you to collaborate on triggers and share resources.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Handle a tool call
pub async fn handle_tool_call(
    client: &AgentAuriClient,
    name: &str,
    arguments: Option<Value>,
) -> ToolCallResult {
    let args = arguments.unwrap_or(json!({}));

    match name {
        "list_triggers" => handle_list_triggers(client, args).await,
        "get_trigger" => handle_get_trigger(client, args).await,
        "create_trigger" => handle_create_trigger(client, args).await,
        "delete_trigger" => handle_delete_trigger(client, args).await,
        "list_linked_agents" => handle_list_linked_agents(client).await,
        "list_following" => handle_list_following(client).await,
        "query_events" => handle_query_events(client, args).await,
        "get_indexer_status" => handle_get_indexer_status(client).await,
        "get_credits" => handle_get_credits(client).await,
        "list_organizations" => handle_list_organizations(client).await,
        _ => ToolCallResult::error(format!("Unknown tool: {}", name)),
    }
}

#[derive(Debug, Deserialize)]
struct ListTriggersArgs {
    page: Option<i32>,
    per_page: Option<i32>,
}

async fn handle_list_triggers(client: &AgentAuriClient, args: Value) -> ToolCallResult {
    let args: ListTriggersArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {}", e)),
    };

    match client.list_triggers(args.page, args.per_page).await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to list triggers: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
struct GetTriggerArgs {
    trigger_id: String,
}

async fn handle_get_trigger(client: &AgentAuriClient, args: Value) -> ToolCallResult {
    let args: GetTriggerArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {}", e)),
    };

    match client.get_trigger(&args.trigger_id).await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to get trigger: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
struct CreateTriggerArgs {
    name: String,
    registry: String,
    event_type: String,
    chain_id: Option<String>,
    enabled: Option<bool>,
}

async fn handle_create_trigger(client: &AgentAuriClient, args: Value) -> ToolCallResult {
    let args: CreateTriggerArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {}", e)),
    };

    let request = CreateTriggerRequest {
        name: args.name,
        registry: args.registry,
        event_type: args.event_type,
        chain_id: args.chain_id,
        enabled: args.enabled,
    };

    match client.create_trigger(request).await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to create trigger: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
struct DeleteTriggerArgs {
    trigger_id: String,
}

async fn handle_delete_trigger(client: &AgentAuriClient, args: Value) -> ToolCallResult {
    let args: DeleteTriggerArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {}", e)),
    };

    match client.delete_trigger(&args.trigger_id).await {
        Ok(()) => ToolCallResult::text("Trigger deleted successfully"),
        Err(e) => ToolCallResult::error(format!("Failed to delete trigger: {}", e)),
    }
}

async fn handle_list_linked_agents(client: &AgentAuriClient) -> ToolCallResult {
    match client.list_linked_agents().await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to list linked agents: {}", e)),
    }
}

async fn handle_list_following(client: &AgentAuriClient) -> ToolCallResult {
    match client.list_following().await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to list following: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
struct QueryEventsArgs {
    event_type: Option<String>,
    limit: Option<i32>,
}

async fn handle_query_events(client: &AgentAuriClient, args: Value) -> ToolCallResult {
    let args: QueryEventsArgs = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolCallResult::error(format!("Invalid arguments: {}", e)),
    };

    match client
        .get_ponder_events(args.event_type.as_deref(), args.limit)
        .await
    {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to query events: {}", e)),
    }
}

async fn handle_get_indexer_status(client: &AgentAuriClient) -> ToolCallResult {
    match client.get_ponder_status().await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to get indexer status: {}", e)),
    }
}

async fn handle_get_credits(client: &AgentAuriClient) -> ToolCallResult {
    match client.get_credits().await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to get credits: {}", e)),
    }
}

async fn handle_list_organizations(client: &AgentAuriClient) -> ToolCallResult {
    match client.list_organizations().await {
        Ok(response) => ToolCallResult::json(&response),
        Err(e) => ToolCallResult::error(format!("Failed to list organizations: {}", e)),
    }
}
