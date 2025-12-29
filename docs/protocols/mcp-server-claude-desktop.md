# AgentAuri MCP Server for Claude Desktop

> **STATUS: IMPLEMENTED - Phase 5**
>
> This MCP server allows Claude Desktop and other MCP-compatible AI clients
> to interact with the AgentAuri API for managing triggers, monitoring agents,
> and querying blockchain events.

## Overview

The AgentAuri MCP server is a native Rust binary that implements the Model Context Protocol (MCP) over stdio. It acts as a bridge between Claude Desktop and the AgentAuri API, enabling natural language interaction with blockchain monitoring infrastructure.

### Architecture

```
┌─────────────────────┐     stdio      ┌──────────────────┐      HTTPS      ┌───────────────────┐
│   Claude Desktop    │ ◄────────────► │  agentauri-mcp   │ ◄─────────────► │ api.agentauri.ai  │
│   (MCP Client)      │   JSON-RPC 2.0 │  (MCP Server)    │    REST API     │   (Backend)       │
└─────────────────────┘                └──────────────────┘                 └───────────────────┘
```

## Installation

### Option 1: Download Pre-built Binary

Download the latest release from GitHub:

```bash
# macOS (Apple Silicon)
curl -L https://github.com/agentauri/api.agentauri.ai/releases/latest/download/agentauri-mcp-darwin-arm64 -o agentauri-mcp
chmod +x agentauri-mcp

# macOS (Intel)
curl -L https://github.com/agentauri/api.agentauri.ai/releases/latest/download/agentauri-mcp-darwin-x64 -o agentauri-mcp
chmod +x agentauri-mcp

# Linux (x64)
curl -L https://github.com/agentauri/api.agentauri.ai/releases/latest/download/agentauri-mcp-linux-x64 -o agentauri-mcp
chmod +x agentauri-mcp
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/agentauri/api.agentauri.ai
cd api.agentauri.ai/rust-backend

# Build release binary
cargo build -p mcp-server --release

# Binary location
ls target/release/agentauri-mcp
```

## Configuration

### Claude Desktop Setup

1. Open Claude Desktop settings
2. Navigate to the MCP Servers section
3. Add a new server configuration

**Config file location:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

**Example configuration:**

```json
{
  "mcpServers": {
    "agentauri": {
      "command": "/path/to/agentauri-mcp",
      "env": {
        "AGENTAURI_API_KEY": "sk_live_your_key_here"
      }
    }
  }
}
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AGENTAURI_API_KEY` | Yes | - | Your API key (sk_live_xxx or sk_test_xxx) |
| `AGENTAURI_API_URL` | No | https://api.agentauri.ai | API endpoint |
| `RUST_LOG` | No | info | Log level (trace, debug, info, warn, error) |

### Getting an API Key

1. Sign up at [app.agentauri.ai](https://app.agentauri.ai)
2. Navigate to Settings > API Keys
3. Create a new API key with appropriate permissions
4. Copy the key (starts with `sk_live_` or `sk_test_`)

## Available Tools

The MCP server exposes the following tools to Claude:

### Trigger Management

| Tool | Description |
|------|-------------|
| `list_triggers` | List all triggers for your account |
| `get_trigger` | Get details of a specific trigger |
| `create_trigger` | Create a new event-driven trigger |
| `delete_trigger` | Delete an existing trigger |

### Agent Monitoring

| Tool | Description |
|------|-------------|
| `list_linked_agents` | List agents you own (cryptographically verified) |
| `list_following` | List agents you're monitoring |

### Blockchain Data

| Tool | Description |
|------|-------------|
| `query_events` | Query blockchain events from the indexer |
| `get_indexer_status` | Check sync status of all monitored chains |

### Account

| Tool | Description |
|------|-------------|
| `get_credits` | Check your credit balance |
| `list_organizations` | List organizations you belong to |

## Usage Examples

Once configured, you can interact with AgentAuri through natural language in Claude Desktop:

### List Your Triggers

> "What triggers do I have set up?"

Claude will use the `list_triggers` tool and present the results.

### Create a New Trigger

> "Create a trigger to alert me when any new agent is registered on Base"

Claude will use `create_trigger` with:
- registry: "identity"
- event_type: "AgentRegistered"
- chain_id: "8453"

### Check Indexer Status

> "Is the blockchain indexer up to date?"

Claude will use `get_indexer_status` to check sync status.

### Query Recent Events

> "Show me the last 10 agent registrations"

Claude will use `query_events` with event_type: "AgentRegistered" and limit: 10.

## Supported Registries and Event Types

### Identity Registry

| Event Type | Description |
|------------|-------------|
| `AgentRegistered` | New agent registered |
| `MetadataUpdated` | Agent metadata changed |
| `AgentTransferred` | Agent ownership transferred |

### Reputation Registry

| Event Type | Description |
|------------|-------------|
| `NewFeedback` | New feedback submitted |
| `FeedbackUpdated` | Feedback modified |

### Validation Registry

| Event Type | Description |
|------------|-------------|
| `ValidationSubmitted` | New validation submitted |
| `ValidationUpdated` | Validation modified |

## Troubleshooting

### Server Not Starting

Check logs by setting `RUST_LOG=debug`:

```json
{
  "mcpServers": {
    "agentauri": {
      "command": "/path/to/agentauri-mcp",
      "env": {
        "AGENTAURI_API_KEY": "sk_live_xxx",
        "RUST_LOG": "debug"
      }
    }
  }
}
```

### API Authentication Errors

- Verify your API key is correct
- Check the key hasn't expired
- Ensure the key has appropriate permissions

### Connection Refused

- Verify the API URL is correct
- Check your internet connection
- Try using the default URL (https://api.agentauri.ai)

## Security Considerations

1. **API Key Security**: Never share your API key or commit it to version control
2. **Local Binary**: The MCP server runs locally and communicates with Claude Desktop over stdio
3. **HTTPS Only**: All API communication uses HTTPS
4. **No Credential Storage**: The server doesn't persist credentials to disk

## Development

### Running Tests

```bash
cd rust-backend
cargo test -p mcp-server
```

### Debug Mode

```bash
RUST_LOG=debug ./target/release/agentauri-mcp
```

### Testing Manually

You can test the MCP server by sending JSON-RPC messages to stdin:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/agentauri-mcp
```

## Protocol Reference

The MCP server implements:
- **Protocol Version**: 2024-11-05
- **Transport**: stdio (JSON-RPC 2.0 over newline-delimited JSON)
- **Capabilities**: tools

For full MCP protocol documentation, see: https://modelcontextprotocol.io/docs
