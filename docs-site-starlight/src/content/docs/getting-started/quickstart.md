---
title: Quickstart
description: Get started with AgentAuri in 5 minutes
sidebar:
  order: 1
---

Get started with AgentAuri in 5 minutes. This guide walks you through creating your first trigger to monitor blockchain events.

## Prerequisites

- An AgentAuri account
- A Telegram bot (for notifications) or a webhook endpoint

## Step 1: Create an Account

```bash
curl -X POST https://api.agentauri.ai/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "myapp",
    "email": "dev@myapp.com",
    "password": "your-secure-password"
  }'
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "myapp",
  "email": "dev@myapp.com"
}
```

## Step 2: Get Your JWT Token

```bash
curl -X POST https://api.agentauri.ai/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "myapp",
    "password": "your-secure-password"
  }'
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "expires_at": "2024-01-16T12:00:00Z"
}
```

Save this token for subsequent requests.

## Step 3: Create an Organization

```bash
curl -X POST https://api.agentauri.ai/api/v1/organizations \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My AI Agents",
    "slug": "my-ai-agents"
  }'
```

## Step 4: Create Your First Trigger

This trigger monitors agent registrations on the Identity Registry:

```bash
curl -X POST https://api.agentauri.ai/api/v1/triggers \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "New Agent Alert",
    "description": "Notify when new agents are registered",
    "organization_id": "YOUR_ORG_ID",
    "enabled": true
  }'
```

## Step 5: Add a Condition

Filter events to only match agent registrations:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/conditions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "AgentRegistered",
    "chain_id": 11155111,
    "field_filters": {
      "registry_type": "identity"
    }
  }'
```

## Step 6: Add an Action

Send a Telegram notification when the trigger fires:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "config": {
      "bot_token": "YOUR_BOT_TOKEN",
      "chat_id": "YOUR_CHAT_ID",
      "message_template": "🤖 New agent registered: {{agent_id}} on {{chain_name}}"
    }
  }'
```

## You're Done!

Your trigger is now active. When a new agent is registered on the Identity Registry, you'll receive a Telegram notification.

## Next Steps

- [Authentication](/getting-started/authentication) - Learn about JWT and API keys
- [Triggers Guide](/concepts/triggers) - Explore advanced trigger configurations
- [Webhook Integration](/guides/webhook-integration) - Set up REST webhook actions
