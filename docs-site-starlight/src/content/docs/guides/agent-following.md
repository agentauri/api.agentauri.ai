---
title: Agent Following
description: Simplified monitoring for ERC-8004 agents
sidebar:
  order: 6
---

Agent Following provides a streamlined way to monitor on-chain agents across all ERC-8004 registries. Instead of manually creating triggers and conditions, simply follow an agent to receive notifications about all their activities.

## Overview

When you follow an agent, AgentAuri automatically creates and manages triggers for:

- **Identity Registry** - Agent registration, updates, metadata changes
- **Reputation Registry** - New feedback, responses, score changes
- **Validation Registry** - Validation requests and responses

## Quick Start

### Follow an Agent

```bash
curl -X POST "https://api.agentauri.ai/api/v1/agents/123/follow" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "chain_id": 11155111,
    "actions": [
      {
        "action_type": "telegram",
        "config": {
          "bot_token": "YOUR_BOT_TOKEN",
          "chat_id": "YOUR_CHAT_ID"
        }
      }
    ]
  }'
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "agent_id": "123",
  "chain_id": 11155111,
  "organization_id": "org-uuid",
  "enabled": true,
  "actions": [
    {
      "action_type": "telegram",
      "config": {
        "chat_id": "YOUR_CHAT_ID"
      }
    }
  ],
  "trigger_ids": {
    "identity": "trigger-uuid-1",
    "reputation": "trigger-uuid-2",
    "validation": "trigger-uuid-3"
  },
  "created_at": "2026-01-09T10:00:00Z"
}
```

## Endpoints

### List Followed Agents

```bash
curl "https://api.agentauri.ai/api/v1/agents/following" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Update Follow Settings

Update actions or enable/disable following:

```bash
curl -X PUT "https://api.agentauri.ai/api/v1/agents/123/follow" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "actions": [
      {
        "action_type": "webhook",
        "config": {
          "url": "https://your-app.com/webhook",
          "method": "POST"
        }
      }
    ]
  }'
```

### Stop Following

```bash
curl -X DELETE "https://api.agentauri.ai/api/v1/agents/123/follow" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Action Types

### Telegram

```json
{
  "action_type": "telegram",
  "config": {
    "bot_token": "YOUR_BOT_TOKEN",
    "chat_id": "YOUR_CHAT_ID",
    "message_template": "Agent {{agent_id}}: {{event_type}}"
  }
}
```

### Webhook

```json
{
  "action_type": "webhook",
  "config": {
    "url": "https://your-app.com/webhook",
    "method": "POST",
    "headers": {
      "Authorization": "Bearer your-secret"
    }
  }
}
```

### MCP (Model Context Protocol)

```json
{
  "action_type": "mcp",
  "config": {
    "server_url": "https://your-mcp-server.com",
    "tool_name": "agent_update"
  }
}
```

## Event Types Monitored

| Registry | Events |
|----------|--------|
| **Identity** | `Registered`, `UriUpdated`, `MetadataSet`, `Transfer` |
| **Reputation** | `NewFeedback`, `ResponseAppended`, `ScoreUpdated` |
| **Validation** | `ValidationRequested`, `ValidationResponse` |

## Template Variables

Use these variables in message templates:

| Variable | Description |
|----------|-------------|
| `{{agent_id}}` | On-chain agent token ID |
| `{{chain_id}}` | Network chain ID |
| `{{chain_name}}` | Human-readable chain name |
| `{{event_type}}` | Event name |
| `{{block_number}}` | Block where event occurred |
| `{{tx_hash}}` | Transaction hash |
| `{{timestamp}}` | Event timestamp |

## Best Practices

1. **Start with Telegram** - Easy setup for testing
2. **Use templates** - Customize messages for your use case
3. **Monitor billing** - Each notification consumes credits
4. **Disable when not needed** - Save credits by pausing follows

## Comparison with Triggers

| Feature | Agent Following | Manual Triggers |
|---------|-----------------|-----------------|
| Setup complexity | Simple | Advanced |
| Customization | Limited | Full control |
| Coverage | All registries | Per-registry |
| Management | Automatic | Manual |

Use **Agent Following** for quick monitoring. Use **Triggers** when you need fine-grained control over conditions and actions.

## Rate Limits

| Endpoint | Limit |
|----------|-------|
| `POST /agents/{id}/follow` | 10/minute |
| `GET /agents/following` | 60/minute |
| `PUT /agents/{id}/follow` | 30/minute |
| `DELETE /agents/{id}/follow` | 30/minute |
