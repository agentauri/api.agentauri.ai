---
title: Telegram Notifications
description: Set up Telegram notifications for blockchain events
sidebar:
  order: 2
---

This guide shows you how to set up Telegram notifications for blockchain events.

## Overview

AgentAuri can send real-time notifications to Telegram chats, groups, or channels when your triggers fire.

## Prerequisites

- A Telegram account
- A Telegram bot (we'll create one)
- An AgentAuri account with an active trigger

## Step 1: Create a Telegram Bot

1. Open Telegram and search for [@BotFather](https://t.me/BotFather)
2. Send `/newbot`
3. Choose a name for your bot (e.g., "My AgentAuri Alerts")
4. Choose a username (must end in `bot`, e.g., `my_agentauri_alerts_bot`)
5. Save the **bot token** provided

```
Use this token to access the HTTP API:
123456789:ABCdefGHIjklMNOpqrsTUVwxyz
```

## Step 2: Get Your Chat ID

### For Private Chats

1. Start a conversation with your new bot
2. Send any message
3. Visit this URL (replace YOUR_TOKEN):
   ```
   https://api.telegram.org/botYOUR_TOKEN/getUpdates
   ```
4. Find `"chat":{"id":123456789}` in the response

### For Groups

1. Add your bot to the group
2. Send a message in the group mentioning the bot
3. Check getUpdates (URL above)
4. Group IDs are negative numbers (e.g., `-1001234567890`)

### For Channels

1. Add your bot as an administrator
2. Post a message in the channel
3. Check getUpdates
4. Channel IDs start with `-100` (e.g., `-1001234567890`)

## Step 3: Configure the Telegram Action

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "config": {
      "bot_token": "123456789:ABCdefGHIjklMNOpqrsTUVwxyz",
      "chat_id": "-1001234567890",
      "message_template": "🤖 New blockchain event!\n\nType: {{event_type}}\nChain: {{chain_name}}\nAgent: {{data.agent_id}}"
    }
  }'
```

## Message Templates

### Basic Template

```
🔔 {{event_type}}

Chain: {{chain_name}}
Block: {{block_number}}
```

### Detailed Template

```
📣 AgentAuri Alert

Event: {{event_type}}
Network: {{chain_name}} ({{chain_id}})
Block: {{block_number}}
Time: {{timestamp}}

Agent: {{data.agent_id}}
Transaction: {{transaction_hash}}
```

### With Markdown Formatting

```json
{
  "message_template": "*{{event_type}}*\n\n_Chain:_ `{{chain_name}}`\n_Agent:_ `{{data.agent_id}}`\n\n[View on Explorer](https://sepolia.etherscan.io/tx/{{transaction_hash}})",
  "parse_mode": "Markdown"
}
```

### With HTML Formatting

```json
{
  "message_template": "<b>{{event_type}}</b>\n\n<i>Chain:</i> <code>{{chain_name}}</code>\n<i>Agent:</i> <code>{{data.agent_id}}</code>\n\n<a href=\"https://sepolia.etherscan.io/tx/{{transaction_hash}}\">View on Explorer</a>",
  "parse_mode": "HTML"
}
```

## Available Template Variables

| Variable | Example |
|----------|---------|
| `{{event_type}}` | `AgentRegistered` |
| `{{chain_id}}` | `11155111` |
| `{{chain_name}}` | `Ethereum Sepolia` |
| `{{block_number}}` | `12345678` |
| `{{timestamp}}` | `2024-01-15T10:00:00Z` |
| `{{transaction_hash}}` | `0xabc...def` |
| `{{contract_address}}` | `0x1234...5678` |
| `{{data.agent_id}}` | `0xagent...` |
| `{{data.owner}}` | `0xowner...` |
| `{{data.metadata_uri}}` | `ipfs://Qm...` |

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bot_token` | string | required | Telegram bot token |
| `chat_id` | string | required | Target chat/group/channel ID |
| `message_template` | string | required | Message with template variables |
| `parse_mode` | string | none | `Markdown` or `HTML` |
| `disable_notification` | boolean | false | Send silently |
| `disable_web_page_preview` | boolean | false | Don't show link previews |

## Examples

### Agent Registration Alert

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "config": {
      "bot_token": "'$BOT_TOKEN'",
      "chat_id": "'$CHAT_ID'",
      "message_template": "🆕 *New Agent Registered*\n\nAgent: `{{data.agent_id}}`\nOwner: `{{data.owner}}`\nChain: {{chain_name}}\n\n[View Transaction](https://sepolia.etherscan.io/tx/{{transaction_hash}})",
      "parse_mode": "Markdown"
    }
  }'
```

### Reputation Update Alert

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "config": {
      "bot_token": "'$BOT_TOKEN'",
      "chat_id": "'$CHAT_ID'",
      "message_template": "⭐ *Reputation Updated*\n\nAgent: `{{data.agent_id}}`\nNew Score: {{data.score}}\nCategory: {{data.category}}",
      "parse_mode": "Markdown"
    }
  }'
```

### Silent Monitoring (No Notification Sound)

```json
{
  "action_type": "telegram",
  "config": {
    "bot_token": "...",
    "chat_id": "...",
    "message_template": "...",
    "disable_notification": true
  }
}
```

## Troubleshooting

### Bot Not Sending Messages

1. **Verify the bot token** - Test with:
   ```bash
   curl "https://api.telegram.org/bot<TOKEN>/getMe"
   ```

2. **Check the chat ID** - Ensure it's correct and the bot has access

3. **For groups/channels** - Make sure the bot is added and has permission to post

### Wrong Chat ID

- Private chats: Positive number (e.g., `123456789`)
- Groups/Channels: Negative number starting with `-100` (e.g., `-1001234567890`)

### Markdown/HTML Errors

If messages fail with parse errors:
- Check for unescaped special characters
- Ensure proper closing tags (HTML)
- Use `parse_mode: null` to disable formatting

### Rate Limits

Telegram limits:
- 30 messages/second to different chats
- 1 message/second to same chat

AgentAuri queues messages to respect these limits.

## Security Best Practices

1. **Never share your bot token** - Treat it like a password
2. **Use private bots** - Disable "Allow Groups" if not needed
3. **Restrict bot permissions** - Only grant necessary admin rights
4. **Monitor bot activity** - Check for unexpected messages
5. **Rotate tokens periodically** - Use @BotFather to regenerate
