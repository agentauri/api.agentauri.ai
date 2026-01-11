---
title: Triggers
description: Understanding triggers - the core building blocks of AgentAuri
sidebar:
  order: 1
---

Triggers are the core building blocks of AgentAuri. They define what blockchain events to monitor and what actions to take when those events occur.

## Anatomy of a Trigger

A trigger consists of three parts:

```
Trigger
├── Metadata (name, description, enabled)
├── Conditions (what events to match)
└── Actions (what to do when matched)
```

## Creating a Trigger

```bash
curl -X POST https://api.agentauri.ai/api/v1/triggers \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "High Reputation Alert",
    "description": "Notify when agents receive reputation scores above 90",
    "organization_id": "org_123",
    "enabled": true
  }'
```

## Conditions

Conditions define which events should activate the trigger.

### Event Types (ERC-8004 v1.0)

| Event Type | Registry | Description |
|------------|----------|-------------|
| `Registered` | Identity | New agent registered |
| `MetadataSet` | Identity | Agent metadata key-value set |
| `URIUpdated` | Identity | Agent URI updated |
| `Transfer` | Identity | Agent ownership transferred |
| `NewFeedback` | Reputation | New feedback submitted |
| `FeedbackRevoked` | Reputation | Feedback revoked |
| `ResponseAppended` | Reputation | Response added to feedback |
| `ValidationRequest` | Validation | Validation requested (not deployed) |
| `ValidationResponse` | Validation | Validation response (not deployed) |

### Chain IDs

| Network | Chain ID |
|---------|----------|
| Ethereum Sepolia | 11155111 |
| Base Sepolia | 84532 |
| Linea Sepolia | 59141 |

### Adding a Condition

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/conditions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "NewFeedback",
    "chain_id": 11155111,
    "contract_address": "0x8004B663056A597Dffe9eCcC1965A193B7388713",
    "field_filters": {
      "score": { "gte": 90 }
    }
  }'
```

### Field Filters

Filter events based on field values:

| Operator | Description | Example |
|----------|-------------|---------|
| `eq` | Equals | `{"status": {"eq": "active"}}` |
| `ne` | Not equals | `{"status": {"ne": "inactive"}}` |
| `gt` | Greater than | `{"score": {"gt": 50}}` |
| `gte` | Greater than or equal | `{"score": {"gte": 50}}` |
| `lt` | Less than | `{"score": {"lt": 100}}` |
| `lte` | Less than or equal | `{"score": {"lte": 100}}` |
| `in` | In array | `{"chain_id": {"in": [1, 11155111]}}` |
| `contains` | String contains | `{"name": {"contains": "AI"}}` |

### Multiple Conditions

Multiple conditions are evaluated with **AND** logic. The trigger fires only when all conditions match.

## Trigger State

Triggers maintain state to prevent duplicate processing:

```json
{
  "last_processed_block": 12345678,
  "last_processed_at": "2024-01-15T10:00:00Z",
  "total_matches": 42,
  "total_actions_executed": 42
}
```

## Enabling/Disabling

```bash
# Disable a trigger
curl -X PUT "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

Disabled triggers stop matching events but retain their configuration.

## Best Practices

1. **Use specific conditions** - Broad conditions increase processing load
2. **Start with test networks** - Validate triggers on Sepolia before mainnet
3. **Monitor trigger state** - Check for gaps in processed blocks
4. **Use meaningful names** - Makes debugging easier
5. **Document with descriptions** - Future you will thank you

## Example: Complete Trigger Setup

```bash
# 1. Create trigger
TRIGGER=$(curl -s -X POST https://api.agentauri.ai/api/v1/triggers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "New Agent Welcome",
    "organization_id": "'$ORG_ID'"
  }')
TRIGGER_ID=$(echo $TRIGGER | jq -r '.id')

# 2. Add condition
curl -X POST "https://api.agentauri.ai/api/v1/triggers/$TRIGGER_ID/conditions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "Registered",
    "chain_id": 11155111
  }'

# 3. Add action
curl -X POST "https://api.agentauri.ai/api/v1/triggers/$TRIGGER_ID/actions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "config": {
      "bot_token": "'$BOT_TOKEN'",
      "chat_id": "'$CHAT_ID'",
      "message_template": "Welcome new agent: {{agent_id}}"
    }
  }'
```
