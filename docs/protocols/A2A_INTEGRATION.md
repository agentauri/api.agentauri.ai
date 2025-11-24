# A2A Protocol Integration

This document describes the integration of Google's Agent-to-Agent (A2A) protocol for the Pull Layer, enabling agents to query reputation and validation data through a standardized async task interface.

## Overview

The A2A Protocol provides a standardized way for AI agents to communicate with each other. In our system, external agents can query the ERC-8004 backend to retrieve reputation data, validation history, and analytics through a JSON-RPC 2.0 interface.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    External Agent                                │
│                                                                  │
│   ┌───────────────────────────────────────────────────────┐    │
│   │  1. Submit task (query)                               │    │
│   │  2. Poll for status / Subscribe to SSE                │    │
│   │  3. Receive result                                    │    │
│   └───────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    A2A Gateway                                   │
│                                                                  │
│   ┌───────────┐    ┌───────────┐    ┌───────────┐              │
│   │ JSON-RPC  │    │   Task    │    │   SSE     │              │
│   │ Endpoint  │────│  Manager  │────│ Streamer  │              │
│   └───────────┘    └───────────┘    └───────────┘              │
│                          │                                      │
│                   ┌──────▼──────┐                               │
│                   │   Query     │                               │
│                   │  Executor   │                               │
│                   └──────┬──────┘                               │
│                          │                                      │
│         ┌────────────────┼────────────────┐                     │
│         │                │                │                     │
│   ┌─────▼─────┐   ┌──────▼──────┐  ┌─────▼─────┐              │
│   │ MCP Query │   │   Cache     │  │  Payment  │              │
│   │   Tools   │   │  (Redis)    │  │  Gateway  │              │
│   └───────────┘   └─────────────┘  └───────────┘              │
└─────────────────────────────────────────────────────────────────┘
```

## Task Lifecycle

Tasks follow a defined lifecycle with clear state transitions:

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌───────────┐
│ submitted│────▶│ working  │────▶│completed │  or │  failed   │
└──────────┘     └────┬─────┘     └──────────┘     └───────────┘
                      │
                      │ progress updates via SSE
                      ▼
                 ┌──────────┐
                 │ message  │ (streaming output)
                 └──────────┘
```

### Task States

| State | Description |
|-------|-------------|
| `submitted` | Task received, awaiting processing |
| `working` | Task is being processed |
| `completed` | Task finished successfully |
| `failed` | Task failed with error |

## API Endpoints

### JSON-RPC Endpoint

**POST /api/v1/a2a/rpc**

All A2A operations use JSON-RPC 2.0 format.

```json
{
  "jsonrpc": "2.0",
  "method": "tasks/send",
  "params": {
    "task": {
      "tool": "getReputationSummary",
      "arguments": {
        "agentId": 42,
        "period": "30d"
      }
    }
  },
  "id": "request-123"
}
```

### Task Status

**GET /api/v1/a2a/tasks/:id**

```json
{
  "id": "task-abc123",
  "status": "working",
  "progress": 0.45,
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:00:05Z"
}
```

### SSE Progress Stream

**GET /api/v1/a2a/tasks/:id/stream**

Server-Sent Events for real-time progress updates:

```
event: progress
data: {"task_id": "task-abc123", "progress": 0.25, "message": "Fetching feedback data..."}

event: progress
data: {"task_id": "task-abc123", "progress": 0.75, "message": "Analyzing patterns..."}

event: complete
data: {"task_id": "task-abc123", "result": {...}}
```

## JSON-RPC Methods

### tasks/send

Submit a new query task.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tasks/send",
  "params": {
    "task": {
      "tool": "getReputationSummary",
      "arguments": {
        "agentId": 42,
        "period": "30d"
      }
    },
    "meta": {
      "organization_id": "org-123",
      "payment_method": "credits"
    }
  },
  "id": "1"
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "task-abc123",
    "status": "submitted",
    "estimated_cost": "0.01 USDC"
  },
  "id": "1"
}
```

### tasks/get

Get task status and result.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tasks/get",
  "params": {
    "task_id": "task-abc123"
  },
  "id": "2"
}
```

**Response** (completed):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "task-abc123",
    "status": "completed",
    "result": {
      "agentId": 42,
      "averageScore": 87.5,
      "totalFeedbacks": 156,
      "positiveRatio": 0.92
    },
    "cost": "0.01 USDC",
    "duration_ms": 245
  },
  "id": "2"
}
```

### tasks/cancel

Cancel a pending or working task.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tasks/cancel",
  "params": {
    "task_id": "task-abc123"
  },
  "id": "3"
}
```

## Database Schema

### A2A Tasks Table

```sql
CREATE TABLE a2a_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    tool TEXT NOT NULL,
    arguments JSONB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('submitted', 'working', 'completed', 'failed', 'cancelled')),
    progress DECIMAL(3, 2) DEFAULT 0,
    result JSONB,
    error TEXT,
    cost DECIMAL(20, 8),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_a2a_tasks_org ON a2a_tasks(organization_id);
CREATE INDEX idx_a2a_tasks_status ON a2a_tasks(status) WHERE status IN ('submitted', 'working');
CREATE INDEX idx_a2a_tasks_created ON a2a_tasks(created_at DESC);
```

### API Keys Table

```sql
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,  -- bcrypt hash of API key
    name TEXT NOT NULL,
    prefix TEXT NOT NULL,    -- First 8 chars for identification
    permissions JSONB NOT NULL DEFAULT '["read"]',
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (prefix)
);

CREATE INDEX idx_api_keys_prefix ON api_keys(prefix);
CREATE INDEX idx_api_keys_org ON api_keys(organization_id);
```

## Authentication

### API Key Authentication

External agents authenticate using API keys:

```
Authorization: Bearer ak_8004_abc123...
```

API keys are:
- Prefixed with `ak_8004_` for easy identification
- Hashed with bcrypt before storage
- Scoped to organizations
- Support expiration dates

### Rate Limiting

| Plan | Requests/minute | Concurrent tasks |
|------|----------------|------------------|
| Starter | 10 | 2 |
| Pro | 100 | 10 |
| Enterprise | 1000 | 50 |

## Agent Card (Discovery)

Agents can discover our capabilities through the Agent Card at `/.well-known/agent.json`:

```json
{
  "name": "ERC-8004 Reputation API",
  "version": "1.0.0",
  "description": "Query agent reputation and validation data from ERC-8004 registries",
  "protocols": ["a2a", "mcp"],
  "endpoints": {
    "a2a": "https://api.8004.dev/api/v1/a2a/rpc",
    "mcp": "https://api.8004.dev/mcp"
  },
  "capabilities": {
    "tools": [
      {
        "name": "getMyFeedbacks",
        "tier": 0,
        "description": "Get all feedbacks for an agent"
      },
      {
        "name": "getReputationSummary",
        "tier": 1,
        "description": "Get aggregated reputation metrics"
      },
      {
        "name": "getReputationReport",
        "tier": 3,
        "description": "AI-generated reputation analysis"
      }
    ]
  },
  "pricing": {
    "currency": "USDC",
    "tiers": {
      "0": "0.001",
      "1": "0.01",
      "2": "0.05",
      "3": "0.20"
    }
  },
  "authentication": {
    "type": "bearer",
    "signup_url": "https://8004.dev/signup"
  }
}
```

## Error Handling

### JSON-RPC Errors

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid JSON-RPC request |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Server error |
| -32001 | Insufficient credits | Not enough credits for query |
| -32002 | Rate limited | Too many requests |
| -32003 | Task not found | Unknown task ID |
| -32004 | Task expired | Task result expired |

**Error Response Example**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Insufficient credits",
    "data": {
      "required": "0.05 USDC",
      "available": "0.02 USDC",
      "purchase_url": "https://8004.dev/billing"
    }
  },
  "id": "1"
}
```

## Implementation Timeline

### Week 16: A2A Protocol Foundation
- Implement JSON-RPC 2.0 endpoint
- Create `a2a_tasks` table
- Basic task submission and retrieval
- Task state management

### Week 16: SSE Streaming
- Implement Server-Sent Events
- Progress tracking for long-running queries
- Connection management

### Week 17: Integration with Query Tools
- Connect A2A to MCP Query Tools
- Implement all Tier 0-2 tools
- Add caching layer

### Week 18: Full Payment Integration
- Integrate with payment gateway
- Credit deduction for queries
- x402 payment support

## Example: Complete Query Flow

```bash
# 1. Submit query
curl -X POST https://api.8004.dev/api/v1/a2a/rpc \
  -H "Authorization: Bearer ak_8004_abc123..." \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tasks/send",
    "params": {
      "task": {
        "tool": "getReputationReport",
        "arguments": {"agentId": 42}
      }
    },
    "id": "1"
  }'

# Response: {"jsonrpc":"2.0","result":{"task_id":"task-xyz789","status":"submitted"},"id":"1"}

# 2. Stream progress (optional)
curl -N https://api.8004.dev/api/v1/a2a/tasks/task-xyz789/stream \
  -H "Authorization: Bearer ak_8004_abc123..."

# 3. Get result
curl https://api.8004.dev/api/v1/a2a/rpc \
  -H "Authorization: Bearer ak_8004_abc123..." \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tasks/get",
    "params": {"task_id": "task-xyz789"},
    "id": "2"
  }'
```

## Related Documentation

- [Payment System](../payments/PAYMENT_SYSTEM.md)
- [Query Tools](../api/QUERY_TOOLS.md)
- [MCP Integration](./mcp-integration.md)
- [Google A2A Spec](https://github.com/google/a2a-protocol)

---

**Last Updated**: November 24, 2024
