# Trigger Examples

## Overview

This document provides practical examples of trigger configurations for common use cases. Each example includes the trigger JSON configuration and expected behavior.

## Simple Event-Driven Triggers

### Example 1: Low Reputation Score Alert

Alert when an agent receives a reputation score below 60.

```json
{
  "name": "Low Score Alert for Agent #42",
  "description": "Notify via Telegram when agent receives score < 60",
  "chain_id": 84532,
  "registry": "reputation",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "score_threshold",
      "field": "score",
      "operator": "<",
      "value": "60"
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "⚠️ Agent #{{agent_id}} received low score: {{score}}/100\nFrom: {{client_address}}\nTags: {{tag1}}, {{tag2}}\nBlock: {{block_number}}",
        "parse_mode": "Markdown"
      }
    }
  ]
}
```

### Example 2: New Agent Registration Tracker

Track all new agent registrations on a specific chain.

```json
{
  "name": "New Agent Registrations on Base Sepolia",
  "description": "Monitor all new agent registrations",
  "chain_id": 84532,
  "registry": "identity",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "event_type_equals",
      "field": "event_type",
      "operator": "=",
      "value": "AgentRegistered"
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "987654321",
        "message_template": "🆕 New Agent Registered!\nAgent ID: {{agent_id}}\nOwner: {{owner}}\nToken URI: {{token_uri}}\nTx: https://sepolia.basescan.org/tx/{{transaction_hash}}"
      }
    }
  ]
}
```

### Example 3: Tag-Specific Feedback

Alert only when feedback contains specific tags (e.g., "refund" requests).

```json
{
  "name": "Refund Request Alerts",
  "description": "Notify when feedback is tagged with 'refund'",
  "chain_id": 84532,
  "registry": "reputation",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "tag_equals",
      "field": "tag1",
      "operator": "=",
      "value": "refund"
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "💰 Refund Request Received!\nAgent: {{agent_id}}\nScore: {{score}}\nClient: {{client_address}}\nFile: {{file_uri}}"
      }
    },
    {
      "action_type": "rest",
      "priority": 2,
      "config": {
        "method": "POST",
        "url": "https://api.myapp.com/refund-alerts",
        "headers": {
          "Authorization": "Bearer YOUR_API_KEY",
          "Content-Type": "application/json"
        },
        "body_template": {
          "agent_id": "{{agent_id}}",
          "score": "{{score}}",
          "client": "{{client_address}}",
          "file_uri": "{{file_uri}}"
        }
      }
    }
  ]
}
```

## Validation Triggers

### Example 4: Trusted Validator Whitelist

Only alert when specific trusted validators return low validation scores.

```json
{
  "name": "Trusted Validator Failure Alert",
  "description": "Alert when trusted validators return response < 80",
  "chain_id": 80002,
  "registry": "validation",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "validator_whitelist",
      "field": "validator_address",
      "operator": "IN",
      "value": "[\"0xValidator1...\", \"0xValidator2...\", \"0xValidator3...\"]"
    },
    {
      "condition_type": "response_threshold",
      "field": "response",
      "operator": "<",
      "value": "80"
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "987654321",
        "message_template": "🚨 Validation Failed!\nAgent: {{agent_id}}\nValidator: {{validator_address}}\nResponse: {{response}}/100\nTag: {{tag}}"
      }
    }
  ]
}
```

## Stateful Triggers

### Example 5: Exponential Moving Average (EMA)

Alert when the 10-feedback exponential moving average drops below 70.

```json
{
  "name": "EMA Score Drop Alert",
  "description": "Alert when 10-feedback EMA drops below 70",
  "chain_id": 84532,
  "registry": "reputation",
  "enabled": true,
  "is_stateful": true,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "ema_threshold",
      "field": "score",
      "operator": "<",
      "value": "70",
      "config": {
        "window_size": 10,
        "alpha": 0.2
      }
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "📉 Agent #{{agent_id}} EMA Score Dropped!\nCurrent EMA: {{ema_score}}\nLatest Score: {{score}}\nThreshold: 70"
      }
    }
  ]
}
```

### Example 6: Spam Detection (Rate Limit)

Alert if agent receives more than 10 negative feedbacks per hour (possible spam attack).

```json
{
  "name": "Spam Feedback Detection",
  "description": "Alert if >10 negative feedbacks per hour",
  "chain_id": 84532,
  "registry": "reputation",
  "enabled": true,
  "is_stateful": true,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "score_threshold",
      "field": "score",
      "operator": "<",
      "value": "50"
    },
    {
      "condition_type": "rate_limit",
      "field": "feedback_count",
      "operator": ">",
      "value": "10",
      "config": {
        "time_window": "1h",
        "reset_on_trigger": true
      }
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "⚠️ SPAM ALERT!\nAgent #{{agent_id}} received {{count}} negative feedbacks in 1 hour!\nPossible attack or service issue."
      }
    }
  ]
}
```

## MCP Integration Triggers

### Example 7: Push Verified Feedback to Agent

Push verified feedback with file content to agent's MCP server.

```json
{
  "name": "Verified Feedback to Agent MCP",
  "description": "Push verified feedback to agent's MCP server",
  "chain_id": 84532,
  "registry": "reputation",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "file_uri_exists",
      "field": "file_uri",
      "operator": "IS NOT NULL",
      "value": ""
    }
  ],
  "actions": [
    {
      "action_type": "mcp",
      "priority": 1,
      "config": {
        "resolve_endpoint": true,
        "tool_name": "agent.receiveFeedback",
        "verify_file_hash": true,
        "include_file_content": true,
        "validate_oasf": true,
        "payload_template": {
          "score": "{{score}}",
          "tag1": "{{tag1}}",
          "tag2": "{{tag2}}",
          "clientAddress": "{{client_address}}",
          "feedbackIndex": "{{feedback_index}}",
          "fileUri": "{{file_uri}}",
          "fileHash": "{{file_hash}}",
          "fileContent": "{{verified_file_content}}",
          "blockNumber": "{{block_number}}",
          "timestamp": "{{timestamp}}"
        }
      }
    }
  ]
}
```

### Example 8: Validation Results to Agent

Push validation results to agent's MCP server for self-improvement.

```json
{
  "name": "Validation Results to Agent",
  "description": "Notify agent of validation results via MCP",
  "chain_id": 84532,
  "registry": "validation",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    }
  ],
  "actions": [
    {
      "action_type": "mcp",
      "priority": 1,
      "config": {
        "resolve_endpoint": true,
        "tool_name": "agent.receiveValidation",
        "verify_file_hash": true,
        "include_file_content": true,
        "payload_template": {
          "validatorAddress": "{{validator_address}}",
          "response": "{{response}}",
          "tag": "{{tag}}",
          "responseUri": "{{response_uri}}",
          "fileContent": "{{verified_file_content}}",
          "blockNumber": "{{block_number}}"
        }
      }
    }
  ]
}
```

## Multi-Action Triggers

### Example 9: Comprehensive Alert System

Trigger with multiple actions: Telegram notification, REST webhook, and MCP update.

```json
{
  "name": "Comprehensive Critical Alert",
  "description": "Multi-channel alerts for critical feedback",
  "chain_id": 84532,
  "registry": "reputation",
  "enabled": true,
  "is_stateful": false,
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "score_threshold",
      "field": "score",
      "operator": "<",
      "value": "30"
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "🚨 CRITICAL: Agent #{{agent_id}} received score {{score}}/100"
      }
    },
    {
      "action_type": "rest",
      "priority": 2,
      "config": {
        "method": "POST",
        "url": "https://api.myapp.com/critical-alerts",
        "headers": {
          "Authorization": "Bearer YOUR_API_KEY",
          "Content-Type": "application/json"
        },
        "body_template": {
          "severity": "critical",
          "agent_id": "{{agent_id}}",
          "score": "{{score}}",
          "client": "{{client_address}}",
          "timestamp": "{{timestamp}}"
        }
      }
    },
    {
      "action_type": "mcp",
      "priority": 3,
      "config": {
        "resolve_endpoint": true,
        "tool_name": "agent.receiveFeedback",
        "verify_file_hash": true,
        "include_file_content": true,
        "payload_template": {
          "score": "{{score}}",
          "severity": "critical",
          "clientAddress": "{{client_address}}",
          "fileContent": "{{verified_file_content}}"
        }
      }
    }
  ]
}
```

## Best Practices

### Trigger Naming

- Use descriptive names that explain the purpose
- Include agent ID or specific identifiers when relevant
- Indicate the action type (e.g., "Alert", "Push", "Notify")

### Condition Combinations

- Start with most restrictive conditions (e.g., agent_id)
- Use AND logic by default (all conditions must match)
- Keep conditions simple and testable

### Action Priorities

- Priority 1: Most important (e.g., Telegram notification)
- Priority 2: Secondary (e.g., REST webhook)
- Priority 3: Optional (e.g., MCP update)
- Lower numbers execute first

### Template Variables

Available variables depend on the event type. Common variables:

**Reputation Events**:
- `{{agent_id}}`
- `{{client_address}}`
- `{{score}}`
- `{{tag1}}`, `{{tag2}}`
- `{{file_uri}}`, `{{file_hash}}`
- `{{feedback_index}}`
- `{{block_number}}`, `{{timestamp}}`

**Validation Events**:
- `{{agent_id}}`
- `{{validator_address}}`
- `{{response}}`
- `{{tag}}`
- `{{response_uri}}`, `{{response_hash}}`
- `{{request_hash}}`
- `{{block_number}}`, `{{timestamp}}`

**Identity Events**:
- `{{agent_id}}`
- `{{owner}}`
- `{{token_uri}}`
- `{{metadata_key}}`, `{{metadata_value}}`
- `{{block_number}}`, `{{timestamp}}`

### Testing Triggers

1. Create trigger with `enabled: false`
2. Test conditions with historical events
3. Verify action execution in production environment
4. Enable trigger in production

## Troubleshooting

### Trigger Not Matching

1. Check that `enabled: true`
2. Verify `chain_id` matches event chain
3. Verify `registry` matches event type
4. Check condition values (case-sensitive for strings)
5. Review event data in Event Store

### Action Not Executing

1. Check action priority (lower = higher priority)
2. Verify action configuration (chat_id, URL, endpoint)
3. Check Result Logger for error messages
4. Review circuit breaker state
5. Check rate limits

### MCP Action Failures

1. Verify agent has MCP endpoint in registration file
2. Check MCP bridge service is running
3. Verify file hash matches (if verify_file_hash: true)
4. Check agent MCP server logs
5. Test with `resolve_endpoint: false` and explicit endpoint

## Additional Examples

For more examples, see:
- [API Documentation](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md) - API request/response examples
- [ERC-8004 Integration](../protocols/erc-8004-integration.md) - Event examples
- [MCP Integration](../protocols/mcp-integration.md) - MCP payload examples
