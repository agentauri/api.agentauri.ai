//! AgentAuri MCP Server
//!
//! This MCP server allows Claude Desktop and other MCP clients to interact
//! with the AgentAuri API for managing triggers, monitoring agents, and
//! querying blockchain events.
//!
//! ## Configuration
//!
//! Set the following environment variables:
//! - `AGENTAURI_API_URL`: API endpoint (default: https://api.agentauri.ai)
//! - `AGENTAURI_API_KEY`: Your API key (sk_live_xxx or sk_test_xxx)
//!
//! ## Usage with Claude Desktop
//!
//! Add to your Claude Desktop config (~/Library/Application Support/Claude/claude_desktop_config.json):
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "agentauri": {
//!       "command": "/path/to/agentauri-mcp",
//!       "env": {
//!         "AGENTAURI_API_KEY": "sk_live_your_key_here"
//!       }
//!     }
//!   }
//! }
//! ```

mod client;
mod protocol;
mod tools;

use crate::client::AgentAuriClient;
use crate::protocol::{
    InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
    ServerInfo, ToolCallParams, ToolListResult, ToolsCapability,
};
use crate::tools::{get_tools, handle_tool_call};

use anyhow::Result;
use serde_json::json;
use std::io::{self, BufRead, Write};
use tracing::{debug, error, info, warn};

const SERVER_NAME: &str = "agentauri-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is used for MCP protocol)
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mcp_server=info".parse().unwrap()),
        )
        .init();

    info!("Starting {} v{}", SERVER_NAME, SERVER_VERSION);

    // Load configuration
    dotenvy::dotenv().ok();

    let api_url = std::env::var("AGENTAURI_API_URL")
        .unwrap_or_else(|_| "https://api.agentauri.ai".to_string());
    let api_key = std::env::var("AGENTAURI_API_KEY").ok();

    if api_key.is_none() {
        warn!("AGENTAURI_API_KEY not set - API calls will fail authentication");
    }

    info!(api_url = %api_url, has_api_key = api_key.is_some(), "Configuration loaded");

    // Create API client
    let client = AgentAuriClient::new(api_url, api_key);

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Run the server
    runtime.block_on(run_server(client))
}

async fn run_server(client: AgentAuriClient) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    info!("MCP server ready, waiting for requests on stdin");

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to read line: {}", e);
                break;
            }
        };

        if line.is_empty() {
            continue;
        }

        debug!(line = %line, "Received request");

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                let response = JsonRpcResponse::error(None, -32700, "Parse error");
                send_response(&mut stdout, &response)?;
                continue;
            }
        };

        let response = handle_request(&client, request).await;
        send_response(&mut stdout, &response)?;
    }

    info!("MCP server shutting down");
    Ok(())
}

async fn handle_request(client: &AgentAuriClient, request: JsonRpcRequest) -> JsonRpcResponse {
    debug!(method = %request.method, "Handling request");

    match request.method.as_str() {
        "initialize" => handle_initialize(request),
        "initialized" => handle_initialized(request),
        "tools/list" => handle_tools_list(request),
        "tools/call" => handle_tools_call(client, request).await,
        "notifications/cancelled" => {
            // Acknowledge cancellation notifications
            JsonRpcResponse::success(request.id, json!({}))
        }
        _ => {
            warn!(method = %request.method, "Unknown method");
            JsonRpcResponse::method_not_found(request.id, &request.method)
        }
    }
}

fn handle_initialize(request: JsonRpcRequest) -> JsonRpcResponse {
    let params: InitializeParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(request.id, -32602, format!("Invalid params: {}", e))
            }
        },
        None => {
            return JsonRpcResponse::error(request.id, -32602, "Missing params");
        }
    };

    info!(
        client = %params.client_info.name,
        version = %params.client_info.version,
        protocol = %params.protocol_version,
        "Client initialized"
    );

    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapability {
                list_changed: false,
            },
        },
        server_info: ServerInfo {
            name: SERVER_NAME.to_string(),
            version: SERVER_VERSION.to_string(),
        },
    };

    JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
}

fn handle_initialized(request: JsonRpcRequest) -> JsonRpcResponse {
    info!("Client sent initialized notification");
    // This is a notification, but we respond anyway for safety
    JsonRpcResponse::success(request.id, json!({}))
}

fn handle_tools_list(request: JsonRpcRequest) -> JsonRpcResponse {
    let tools = get_tools();
    let result = ToolListResult { tools };
    JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
}

async fn handle_tools_call(client: &AgentAuriClient, request: JsonRpcRequest) -> JsonRpcResponse {
    let params: ToolCallParams = match request.params {
        Some(p) => match serde_json::from_value(p) {
            Ok(params) => params,
            Err(e) => {
                return JsonRpcResponse::error(request.id, -32602, format!("Invalid params: {}", e))
            }
        },
        None => {
            return JsonRpcResponse::error(request.id, -32602, "Missing params");
        }
    };

    info!(tool = %params.name, "Executing tool");

    let result = handle_tool_call(client, &params.name, params.arguments).await;
    JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
}

fn send_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> Result<()> {
    let json = serde_json::to_string(response)?;
    debug!(response = %json, "Sending response");
    writeln!(stdout, "{}", json)?;
    stdout.flush()?;
    Ok(())
}
