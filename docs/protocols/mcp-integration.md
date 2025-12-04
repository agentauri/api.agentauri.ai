<!-- STATUS: PHASE 5 - NOT YET IMPLEMENTED -->
<!-- This is a design document for Phase 5 (Weeks 16-18) -->

# MCP Integration Guide

## Overview

The Model Context Protocol (MCP) is a standardized protocol for communication between AI agents and external systems. In the api.agentauri.ai backend, MCP serves as the critical bridge for pushing on-chain feedback and validation results to off-chain agents, enabling them to learn and adapt based on their reputation.

## Why MCP Integration is Critical

### The Learning Loop

```
On-Chain Reputation Event (NewFeedback)
    ↓
Backend detects and processes event
    ↓
Backend pushes structured feedback to agent's MCP server
    ↓
Agent receives feedback via MCP tool (agent.receiveFeedback)
    ↓
Agent updates internal model/strategy
    ↓
Agent improves behavior in future interactions
    ↓
Better reputation on-chain (positive feedback loop)
```

### Key Benefits

1. **Standardization**: Uniform interface across heterogeneous agents
2. **Security**: MCP's authentication model protects agent endpoints
3. **Flexibility**: Agents control what tools/resources they expose
4. **Composability**: Enables complex multi-agent workflows
5. **Future-Proofing**: Emerging standard for AI agent communication

## MCP Protocol Basics

### Core Concepts

**MCP Server**: A server that exposes tools, resources, and prompts that clients can use.

**MCP Client**: A client that connects to MCP servers and invokes tools/reads resources.

**Tools**: Functions that the server exposes for clients to call (e.g., `agent.receiveFeedback`).

**Resources**: Data that the server exposes for clients to read (e.g., agent configuration, state).

**Prompts**: Templated messages for AI interactions.

### Protocol Specification

**Official Spec**: https://github.com/modelcontextprotocol/specification

**TypeScript SDK**: https://github.com/modelcontextprotocol/typescript-sdk

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────┐
│  Rust Action Worker (MCP Worker)                    │
│                                                     │
│  1. Receives MCP action job from Redis queue        │
│  2. Resolves agent endpoint from registration file  │
│  3. Fetches and verifies IPFS feedback file         │
│  4. Constructs MCP payload                          │
│  5. Calls MCP Bridge Service via HTTP               │
└────────────────┬────────────────────────────────────┘
                 │ HTTP POST
┌────────────────▼────────────────────────────────────┐
│  TypeScript MCP Bridge Service                      │
│                                                     │
│  1. Receives payload from Rust worker               │
│  2. Creates MCP client with agent endpoint          │
│  3. Authenticates using agent's auth config         │
│  4. Invokes specified MCP tool                      │
│  5. Returns result to Rust worker                   │
└────────────────┬────────────────────────────────────┘
                 │ MCP Protocol
┌────────────────▼────────────────────────────────────┐
│  Agent's MCP Server                                 │
│                                                     │
│  Exposed tools:                                     │
│  - agent.receiveFeedback                            │
│  - agent.receiveValidation                          │
│  - agent.updateConfiguration                        │
└─────────────────────────────────────────────────────┘
```

### Why Rust + TypeScript Bridge?

**Rust Advantages**:
- Event processing and action orchestration (fast, type-safe)
- PostgreSQL and Redis integration
- Consistent with rest of backend

**TypeScript Advantages**:
- Official MCP SDK is TypeScript
- Direct protocol support without reimplementation
- Easy to update when protocol evolves

**Bridge Approach**:
- Rust worker calls TypeScript service via HTTP
- TypeScript service acts as MCP client
- Clean separation of concerns

## MCP Bridge Service Implementation

### Service Architecture

```typescript
// mcp-bridge-service/src/server.ts
import express from 'express';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const app = express();
app.use(express.json());

interface McpCallRequest {
  endpoint: {
    url: string;
    transport: 'stdio' | 'http';
    command?: string;
    args?: string[];
    headers?: Record<string, string>;
  };
  tool_name: string;
  arguments: Record<string, any>;
  timeout_ms?: number;
}

app.post('/mcp/call', async (req, res) => {
  try {
    const request: McpCallRequest = req.body;

    // Create MCP client
    const transport = createTransport(request.endpoint);
    const client = new Client({
      name: 'agentauri-backend',
      version: '1.0.0',
    }, {
      capabilities: {},
    });

    await client.connect(transport);

    // Call tool with timeout
    const timeout = request.timeout_ms || 30000;
    const result = await Promise.race([
      client.callTool({
        name: request.tool_name,
        arguments: request.arguments,
      }),
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error('MCP call timeout')), timeout)
      ),
    ]);

    await client.close();

    res.json({ success: true, result });
  } catch (error) {
    console.error('MCP call failed:', error);
    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

function createTransport(endpoint: any) {
  if (endpoint.transport === 'stdio') {
    return new StdioClientTransport({
      command: endpoint.command,
      args: endpoint.args || [],
    });
  } else if (endpoint.transport === 'http') {
    // HTTP transport implementation
    // (depends on MCP spec for HTTP transport)
    throw new Error('HTTP transport not yet implemented');
  } else {
    throw new Error(`Unsupported transport: ${endpoint.transport}`);
  }
}

const PORT = process.env.PORT || 3001;
app.listen(PORT, () => {
  console.log(`MCP Bridge Service listening on port ${PORT}`);
});
```

### Rust Worker Integration

```rust
// action-workers/src/mcp/worker.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct McpCallRequest {
    endpoint: McpEndpoint,
    tool_name: String,
    arguments: serde_json::Value,
    timeout_ms: u64,
}

#[derive(Debug, Serialize)]
struct McpEndpoint {
    url: Option<String>,
    transport: String,
    command: Option<String>,
    args: Option<Vec<String>>,
    headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct McpCallResponse {
    success: bool,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

pub async fn call_mcp_tool(
    endpoint: McpEndpoint,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value> {
    let client = Client::new();
    let bridge_url = std::env::var("MCP_BRIDGE_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());

    let request = McpCallRequest {
        endpoint,
        tool_name: tool_name.to_string(),
        arguments,
        timeout_ms: 30000,
    };

    let response = client
        .post(format!("{}/mcp/call", bridge_url))
        .json(&request)
        .send()
        .await?;

    let mcp_response: McpCallResponse = response.json().await?;

    if mcp_response.success {
        Ok(mcp_response.result.unwrap_or_default())
    } else {
        Err(anyhow!(
            "MCP call failed: {}",
            mcp_response.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}
```

## Endpoint Discovery

### Registration File Format

Agents declare their MCP server endpoint in their registration file (tokenURI):

```json
{
  "name": "Trading Agent Alpha",
  "version": "1.0.0",
  "capabilities": {
    "mcp": {
      "transport": "stdio",
      "command": "/usr/local/bin/agent-mcp-server",
      "args": ["--agent-id", "42", "--config", "/etc/agent/config.json"],
      "tools": [
        {
          "name": "agent.receiveFeedback",
          "description": "Receive reputation feedback from ERC-8004 backend",
          "inputSchema": {
            "type": "object",
            "properties": {
              "score": {
                "type": "integer",
                "minimum": 0,
                "maximum": 100,
                "description": "Reputation score (0-100)"
              },
              "tag1": {
                "type": "string",
                "description": "Primary feedback tag"
              },
              "tag2": {
                "type": "string",
                "description": "Secondary feedback tag"
              },
              "clientAddress": {
                "type": "string",
                "description": "Ethereum address of feedback provider"
              },
              "feedbackIndex": {
                "type": "integer",
                "description": "Index of this feedback"
              },
              "fileUri": {
                "type": "string",
                "description": "IPFS/HTTPS URI to detailed feedback"
              },
              "fileHash": {
                "type": "string",
                "description": "Hash of feedback file for verification"
              },
              "fileContent": {
                "type": "object",
                "description": "Verified feedback file content"
              },
              "blockNumber": {
                "type": "integer",
                "description": "Block number when feedback was submitted"
              },
              "timestamp": {
                "type": "integer",
                "description": "Unix timestamp of feedback"
              }
            },
            "required": ["score", "clientAddress", "feedbackIndex"]
          }
        },
        {
          "name": "agent.receiveValidation",
          "description": "Receive validation results",
          "inputSchema": {
            "type": "object",
            "properties": {
              "validatorAddress": { "type": "string" },
              "response": { "type": "integer" },
              "tag": { "type": "string" },
              "responseUri": { "type": "string" },
              "fileContent": { "type": "object" }
            },
            "required": ["validatorAddress", "response"]
          }
        }
      ],
      "authentication": {
        "type": "bearer",
        "tokenHeader": "X-Agent-Token",
        "note": "Backend must provide valid token for authentication"
      }
    }
  }
}
```

### Endpoint Resolution Flow

```rust
pub async fn resolve_mcp_endpoint(
    agent_id: u64,
    identity_registry: &IdentityRegistry,
    ipfs_client: &IpfsClient,
) -> Result<McpEndpoint> {
    // 1. Check cache
    if let Some(cached) = MCP_ENDPOINT_CACHE.get(&agent_id) {
        debug!(agent_id, "MCP endpoint found in cache");
        return Ok(cached);
    }

    // 2. Fetch tokenURI from IdentityRegistry contract
    let token_uri = identity_registry.token_uri(agent_id).await
        .context("Failed to fetch tokenURI from IdentityRegistry")?;

    debug!(agent_id, token_uri = %token_uri, "Fetched tokenURI");

    // 3. Fetch registration file
    let registration_file = ipfs_client.fetch(&token_uri).await
        .context("Failed to fetch registration file from IPFS")?;

    // 4. Parse registration file
    let registration: RegistrationFile = serde_json::from_slice(&registration_file)
        .context("Failed to parse registration file JSON")?;

    // 5. Extract MCP endpoint configuration
    let mcp_config = registration.capabilities.mcp
        .ok_or_else(|| anyhow!("Agent does not expose MCP capability"))?;

    let endpoint = McpEndpoint {
        transport: mcp_config.transport,
        command: mcp_config.command,
        args: mcp_config.args,
        url: mcp_config.url,
        headers: mcp_config.authentication.map(|auth| {
            let mut headers = HashMap::new();
            if let Some(token) = get_agent_auth_token(agent_id) {
                headers.insert(auth.token_header, token);
            }
            headers
        }),
    };

    // 6. Cache for future use (invalidate on MetadataSet events)
    MCP_ENDPOINT_CACHE.set(agent_id, endpoint.clone());

    info!(agent_id, "MCP endpoint resolved and cached");

    Ok(endpoint)
}
```

## MCP Action Execution

### Complete Flow

```rust
pub async fn execute_mcp_action(
    action: &TriggerAction,
    event: &Event,
    pool: &PgPool,
) -> Result<ActionResult> {
    let start = Instant::now();

    // 1. Extract configuration
    let config: McpActionConfig = serde_json::from_value(action.config.clone())?;

    // 2. Resolve MCP endpoint (from registration file)
    let endpoint = if config.resolve_endpoint {
        resolve_mcp_endpoint(event.agent_id.unwrap(), &identity_registry, &ipfs_client).await?
    } else {
        // Use explicitly configured endpoint
        config.endpoint.ok_or_else(|| anyhow!("No MCP endpoint configured"))?
    };

    // 3. Fetch and verify IPFS file if configured
    let file_content = if config.include_file_content && event.file_uri.is_some() {
        let uri = event.file_uri.as_ref().unwrap();
        let content = ipfs_client.fetch(uri).await?;

        // Verify hash if configured
        if config.verify_file_hash {
            let expected_hash = event.file_hash.as_ref()
                .ok_or_else(|| anyhow!("No file hash in event"))?;
            let actual_hash = compute_hash(&content);

            if actual_hash != *expected_hash {
                return Err(anyhow!(
                    "File hash mismatch: expected {}, got {}",
                    expected_hash,
                    actual_hash
                ));
            }
        }

        Some(serde_json::from_slice(&content)?)
    } else {
        None
    };

    // 4. Build payload from template
    let mut payload = render_template(&config.payload_template, event)?;

    // Add verified file content if available
    if let Some(content) = file_content {
        payload["fileContent"] = content;
    }

    // 5. Call MCP tool via bridge service
    let result = call_mcp_tool(
        endpoint,
        &config.tool_name,
        payload,
    ).await?;

    let duration = start.elapsed();

    info!(
        agent_id = event.agent_id,
        tool_name = %config.tool_name,
        duration_ms = duration.as_millis(),
        "MCP action executed successfully"
    );

    Ok(ActionResult {
        status: ActionStatus::Success,
        duration_ms: duration.as_millis() as i32,
        response_data: Some(result),
        error_message: None,
    })
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("MCP endpoint not found for agent {0}")]
    EndpointNotFound(u64),

    #[error("MCP tool {0} not exposed by agent")]
    ToolNotFound(String),

    #[error("MCP call timeout after {0}ms")]
    Timeout(u64),

    #[error("MCP authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("MCP bridge service unavailable: {0}")]
    BridgeUnavailable(String),

    #[error("File verification failed: {0}")]
    FileVerification(String),
}

// Retry strategy for transient errors
pub async fn execute_mcp_action_with_retry(
    action: &TriggerAction,
    event: &Event,
    pool: &PgPool,
) -> Result<ActionResult> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        attempts += 1;

        match execute_mcp_action(action, event, pool).await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < max_attempts && is_retryable(&e) => {
                let backoff = Duration::from_secs(2u64.pow(attempts - 1));
                warn!(
                    attempt = attempts,
                    backoff_ms = backoff.as_millis(),
                    error = %e,
                    "MCP action failed, retrying"
                );
                tokio::time::sleep(backoff).await;
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_retryable(error: &anyhow::Error) -> bool {
    // Retry on timeout, bridge unavailable, network errors
    error.downcast_ref::<McpError>().map_or(false, |e| {
        matches!(
            e,
            McpError::Timeout(_) | McpError::BridgeUnavailable(_)
        )
    })
}
```

## Authentication

### Bearer Token Authentication

Many agents use bearer token authentication for MCP calls. The backend must securely store and provide these tokens.

**Token Storage**:

```sql
CREATE TABLE agent_mcp_tokens (
    agent_id BIGINT PRIMARY KEY,
    token TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_agent FOREIGN KEY (agent_id) REFERENCES events(agent_id)
);
```

**Token Retrieval**:

```rust
pub async fn get_agent_auth_token(agent_id: u64, pool: &PgPool) -> Result<Option<String>> {
    let token = sqlx::query!(
        "SELECT token FROM agent_mcp_tokens WHERE agent_id = $1",
        agent_id as i64
    )
    .fetch_optional(pool)
    .await?;

    Ok(token.map(|row| row.token))
}
```

**Security Considerations**:
- Tokens should be encrypted at rest
- Rotate tokens periodically
- Use environment variables or secret manager for sensitive tokens
- Never log tokens

## Testing

### Unit Tests

Mock MCP bridge service responses:

```rust
#[tokio::test]
async fn test_mcp_action_execution() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/mcp/call"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "result": {
                "acknowledged": true,
                "feedback_id": "fb_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Set mock server URL
    std::env::set_var("MCP_BRIDGE_URL", mock_server.uri());

    // Execute action
    let result = execute_mcp_action(&action, &event, &pool).await.unwrap();

    assert_eq!(result.status, ActionStatus::Success);
}
```

### Integration Tests

Run actual MCP bridge service with test agent MCP server:

```typescript
// test-agent-mcp-server.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

const server = new Server({
  name: 'test-agent',
  version: '1.0.0',
}, {
  capabilities: {
    tools: {},
  },
});

server.setRequestHandler('tools/list', async () => ({
  tools: [{
    name: 'agent.receiveFeedback',
    description: 'Test feedback receiver',
    inputSchema: {
      type: 'object',
      properties: {
        score: { type: 'integer' },
      },
    },
  }],
}));

server.setRequestHandler('tools/call', async (request) => {
  if (request.params.name === 'agent.receiveFeedback') {
    return {
      content: [{
        type: 'text',
        text: JSON.stringify({
          acknowledged: true,
          score_received: request.params.arguments.score,
        }),
      }],
    };
  }
});

const transport = new StdioServerTransport();
server.connect(transport);
```

## Monitoring

### Metrics

```rust
// Prometheus metrics
lazy_static! {
    static ref MCP_CALLS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "mcp_calls_total",
        "Total number of MCP calls",
        &["tool_name", "status"]
    ).unwrap();

    static ref MCP_CALL_DURATION: HistogramVec = register_histogram_vec!(
        "mcp_call_duration_seconds",
        "MCP call duration in seconds",
        &["tool_name"],
        vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0]
    ).unwrap();
}
```

### Logging

```rust
#[instrument(skip(action, event, pool), fields(
    agent_id = %event.agent_id.unwrap(),
    tool_name = %config.tool_name
))]
pub async fn execute_mcp_action(...) {
    info!("Executing MCP action");

    // ... execution logic ...

    match result {
        Ok(_) => info!("MCP action succeeded"),
        Err(e) => error!(error = %e, "MCP action failed"),
    }
}
```

## Best Practices

1. **Cache endpoint configurations**: Avoid fetching registration files on every action
2. **Verify file hashes**: Always verify IPFS file integrity before sending to agents
3. **Implement timeouts**: MCP calls should timeout after 30s to prevent hanging
4. **Handle authentication carefully**: Never log tokens, use secure storage
5. **Retry transient failures**: Network errors and timeouts should be retried
6. **Monitor MCP health**: Track success rates and latency per tool
7. **Validate schemas**: Ensure payloads match agent's inputSchema

## Future Enhancements

1. **HTTP Transport**: Add support for HTTP-based MCP connections (when spec is finalized)
2. **Bidirectional Communication**: Enable agents to push updates back to backend
3. **Resource Updates**: Support updating MCP resources (not just calling tools)
4. **Batch Operations**: Group multiple feedback events into single MCP call
5. **Agent SDK**: Provide SDK for agents to easily implement MCP servers

## References

- **MCP Specification**: https://github.com/modelcontextprotocol/specification
- **TypeScript SDK**: https://github.com/modelcontextprotocol/typescript-sdk
- **MCP Documentation**: https://modelcontextprotocol.io/docs
