---
title: Webhook Integration
description: Set up REST webhook actions to integrate AgentAuri with your existing systems
sidebar:
  order: 1
---

This guide walks you through setting up REST webhook actions to integrate AgentAuri with your existing systems.

## Overview

Webhooks allow AgentAuri to push real-time blockchain events to your HTTP endpoints.

```
AgentAuri Event Processor
        │
        ▼
    Your Webhook Endpoint
        │
        ▼
    Your Application Logic
```

## Step 1: Create Your Webhook Endpoint

Your endpoint should:
- Accept POST requests
- Return 2xx status on success
- Process requests within 30 seconds

### Example: Express.js

```javascript
const express = require('express');
const crypto = require('crypto');
const app = express();

app.use(express.json());

// Verify webhook signature (recommended)
function verifySignature(payload, signature, secret) {
  const expected = crypto
    .createHmac('sha256', secret)
    .update(JSON.stringify(payload))
    .digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expected)
  );
}

app.post('/webhook/agentauri', (req, res) => {
  const signature = req.headers['x-agentauri-signature'];

  // Verify signature
  if (!verifySignature(req.body, signature, process.env.WEBHOOK_SECRET)) {
    return res.status(401).send('Invalid signature');
  }

  const { trigger_id, event, timestamp } = req.body;

  console.log(`Received ${event.event_type} from trigger ${trigger_id}`);

  // Process the event
  handleEvent(event);

  res.status(200).json({ received: true });
});

app.listen(3000);
```

### Example: Python Flask

```python
from flask import Flask, request, jsonify
import hmac
import hashlib
import os

app = Flask(__name__)

def verify_signature(payload, signature, secret):
    expected = hmac.new(
        secret.encode(),
        payload.encode(),
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(signature, expected)

@app.route('/webhook/agentauri', methods=['POST'])
def webhook():
    signature = request.headers.get('X-AgentAuri-Signature', '')

    if not verify_signature(request.data.decode(), signature, os.environ['WEBHOOK_SECRET']):
        return jsonify({'error': 'Invalid signature'}), 401

    data = request.json
    event = data['event']

    print(f"Received {event['event_type']}")

    # Process the event
    handle_event(event)

    return jsonify({'received': True})
```

## Step 2: Configure the Webhook Action

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "rest",
    "config": {
      "url": "https://api.yourapp.com/webhook/agentauri",
      "method": "POST",
      "headers": {
        "Authorization": "Bearer your-webhook-secret",
        "X-Custom-Header": "custom-value"
      },
      "timeout_ms": 10000,
      "retry_count": 3
    }
  }'
```

## Step 3: Custom Payload Templates

Transform the event data to match your system's expectations:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/actions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "rest",
    "config": {
      "url": "https://api.yourapp.com/webhook",
      "body_template": {
        "type": "blockchain_event",
        "source": "agentauri",
        "payload": {
          "kind": "{{event_type}}",
          "network": "{{chain_name}}",
          "block": {{block_number}},
          "tx": "{{transaction_hash}}",
          "agent": "{{data.agent_id}}"
        },
        "received_at": "{{timestamp}}"
      }
    }
  }'
```

## Webhook Payload Structure

### Default Payload

When no `body_template` is specified:

```json
{
  "trigger_id": "trig_abc123",
  "trigger_name": "Agent Registration Alert",
  "event": {
    "id": "evt_xyz789",
    "event_type": "AgentRegistered",
    "chain_id": 11155111,
    "chain_name": "Ethereum Sepolia",
    "block_number": 12345678,
    "block_timestamp": "2024-01-15T10:00:00Z",
    "transaction_hash": "0xabc...def",
    "contract_address": "0x1234...5678",
    "data": {
      "agent_id": "0xagent...",
      "owner": "0xowner...",
      "metadata_uri": "ipfs://Qm..."
    }
  },
  "timestamp": "2024-01-15T10:00:05Z"
}
```

### Response Headers

AgentAuri sends these headers with every webhook:

| Header | Description |
|--------|-------------|
| `Content-Type` | `application/json` |
| `X-AgentAuri-Trigger-Id` | Trigger that fired |
| `X-AgentAuri-Event-Id` | Unique event ID |
| `X-AgentAuri-Timestamp` | Request timestamp |
| `X-AgentAuri-Signature` | HMAC-SHA256 signature (if secret configured) |

## Handling Failures

### Retry Logic

Failed webhooks are retried with exponential backoff:

| Attempt | Delay | Cumulative |
|---------|-------|------------|
| 1 | 0s | 0s |
| 2 | 1s | 1s |
| 3 | 5s | 6s |
| 4 (final) | 30s | 36s |

### Failure Conditions

A webhook is considered failed when:
- HTTP status code ≥ 400
- Connection timeout (default: 30s)
- DNS resolution failure
- SSL/TLS errors

### Dead Letter Queue

After all retries fail, the event is moved to a dead letter queue. Monitor and reprocess failed events via the API:

```bash
# List failed events
curl "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/dlq" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"

# Replay a failed event
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/dlq/EVENT_ID/replay" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Best Practices

### Security

1. **Use HTTPS** - Never expose webhook endpoints over HTTP
2. **Verify signatures** - Always validate `X-AgentAuri-Signature`
3. **Use secrets** - Store webhook secrets in environment variables
4. **Allowlist IPs** - AgentAuri sends from documented IP ranges

### Reliability

1. **Return quickly** - Process asynchronously if > 5 seconds
2. **Be idempotent** - Handle duplicate deliveries gracefully
3. **Log everything** - Track event IDs for debugging
4. **Monitor failures** - Set up alerts for webhook errors

### Performance

1. **Use connection pooling** - Reuse HTTP connections
2. **Queue internally** - Decouple receipt from processing
3. **Set reasonable timeouts** - 10-30 seconds recommended

## Debugging

### Testing Your Endpoint

Use curl to simulate a webhook:

```bash
curl -X POST https://your-endpoint.com/webhook \
  -H "Content-Type: application/json" \
  -H "X-AgentAuri-Trigger-Id: test" \
  -d '{
    "trigger_id": "test",
    "event": {
      "event_type": "AgentRegistered",
      "chain_id": 11155111,
      "data": {"agent_id": "0xtest"}
    },
    "timestamp": "2024-01-15T10:00:00Z"
  }'
```

### Webhook Logs

View recent webhook deliveries:

```bash
curl "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/logs?action_type=rest" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```
