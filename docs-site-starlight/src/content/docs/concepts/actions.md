---
title: Actions
description: Understanding action types and configuration
sidebar:
  order: 2
---

Actions define what happens when a trigger's conditions are met. AgentAuri supports multiple action types for different integration scenarios.

## Action Types

| Type | Description | Use Case |
|------|-------------|----------|
| `telegram` | Send Telegram message | Real-time alerts |
| `rest` | HTTP webhook | System integrations |
| `mcp` | MCP server update | AI agent workflows |

## Telegram Actions

Send notifications to Telegram chats or channels.

### Configuration

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "config": {
      "bot_token": "123456789:ABCdefGHIjklMNOpqrsTUVwxyz",
      "chat_id": "-1001234567890",
      "message_template": "🚨 Alert: {{event_type}} on {{chain_name}}"
    }
  }'
```

### Config Fields

| Field | Required | Description |
|-------|----------|-------------|
| `bot_token` | Yes | Telegram bot token from @BotFather |
| `chat_id` | Yes | Target chat/channel ID |
| `message_template` | Yes | Message with template variables |
| `parse_mode` | No | `HTML` or `Markdown` (default: none) |
| `disable_notification` | No | Silent message (default: false) |

### Getting a Bot Token

1. Message [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot` and follow prompts
3. Copy the token provided

### Getting a Chat ID

For private chats: Message your bot, then check:
```
https://api.telegram.org/bot<TOKEN>/getUpdates
```

For groups/channels: Add the bot, send a message, then check getUpdates.

## REST Webhook Actions

Send HTTP requests to external endpoints.

### Configuration

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "rest",
    "config": {
      "url": "https://api.yourservice.com/webhook",
      "method": "POST",
      "headers": {
        "Authorization": "Bearer your-webhook-secret",
        "Content-Type": "application/json"
      },
      "body_template": {
        "event": "{{event_type}}",
        "agent_id": "{{agent_id}}",
        "chain_id": {{chain_id}},
        "timestamp": "{{timestamp}}"
      }
    }
  }'
```

### Config Fields

| Field | Required | Description |
|-------|----------|-------------|
| `url` | Yes | Webhook endpoint URL |
| `method` | No | HTTP method (default: POST) |
| `headers` | No | Custom HTTP headers |
| `body_template` | No | JSON body with template variables |
| `timeout_ms` | No | Request timeout (default: 30000) |
| `retry_count` | No | Retry attempts (default: 3) |

### Webhook Payload

If no `body_template` is specified, the full event is sent:

```json
{
  "trigger_id": "trig_abc123",
  "event": {
    "id": "evt_xyz789",
    "event_type": "AgentRegistered",
    "chain_id": 11155111,
    "block_number": 12345678,
    "transaction_hash": "0x...",
    "data": {
      "agent_id": "0x...",
      "metadata_uri": "ipfs://..."
    }
  },
  "timestamp": "2024-01-15T10:00:00Z"
}
```

## MCP Actions

Update Model Context Protocol servers for AI agent integration.

### Configuration

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "mcp",
    "config": {
      "server_url": "https://mcp.yourservice.com",
      "resource_type": "agent_registry",
      "operation": "update"
    }
  }'
```

## Template Variables

Use these variables in message and body templates:

| Variable | Description |
|----------|-------------|
| `{{event_type}}` | Type of event (AgentRegistered, etc.) |
| `{{chain_id}}` | Blockchain chain ID |
| `{{chain_name}}` | Human-readable chain name |
| `{{block_number}}` | Block where event occurred |
| `{{transaction_hash}}` | Transaction hash |
| `{{timestamp}}` | ISO 8601 timestamp |
| `{{agent_id}}` | Agent address (if applicable) |
| `{{contract_address}}` | Registry contract address |

Access nested data with dot notation: `{{data.score}}`, `{{data.metadata_uri}}`

## Action Execution

### Retry Logic

Failed actions are retried with exponential backoff:

| Attempt | Delay |
|---------|-------|
| 1 | Immediate |
| 2 | 1 second |
| 3 | 5 seconds |
| 4 | 30 seconds |

### Dead Letter Queue

After all retries fail, actions are moved to a dead letter queue for investigation.

### Execution Order

Multiple actions on a trigger execute **in parallel** for speed. Use separate triggers if you need sequential execution.

## Best Practices

1. **Use HTTPS** - Always use secure endpoints for webhooks
2. **Authenticate webhooks** - Include secrets in headers
3. **Handle idempotency** - Webhooks may be delivered multiple times
4. **Monitor failures** - Check dead letter queue regularly
5. **Test thoroughly** - Use test networks before production
