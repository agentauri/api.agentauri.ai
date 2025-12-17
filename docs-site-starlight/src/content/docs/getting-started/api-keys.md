---
title: API Keys
description: Manage API keys for secure server-to-server integrations
sidebar:
  order: 3
---

API keys provide secure, scoped access for server-to-server integrations.

## Key Format

AgentAuri API keys follow this format:

```
sk_test_abc123def456ghi789jkl012mno345
```

- `sk_` - Secret key prefix
- `test_` or `live_` - Environment indicator
- 32-character random string

## Creating API Keys

```bash
curl -X POST https://api.agentauri.ai/api/v1/api-keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Webhook Server",
    "organization_id": "org_123",
    "scopes": ["triggers:read", "triggers:write", "events:read"]
  }'
```

Response:
```json
{
  "id": "key_abc123def456",
  "key": "sk_test_abc123def456ghi789jkl012mno345pqr",
  "name": "Production Webhook Server",
  "organization_id": "org_123",
  "scopes": ["triggers:read", "triggers:write", "events:read"],
  "created_at": "2024-01-15T10:00:00Z",
  "last_used_at": null
}
```

:::caution[Important]
The `key` field is only returned once at creation. Store it securely immediately.
:::

## Available Scopes

| Scope | Description |
|-------|-------------|
| `triggers:read` | List and view triggers |
| `triggers:write` | Create, update, delete triggers |
| `events:read` | Query blockchain events |
| `organizations:read` | View organization details |
| `organizations:write` | Manage organization settings |
| `api-keys:manage` | Create and revoke API keys |

## Listing API Keys

```bash
curl https://api.agentauri.ai/api/v1/api-keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

Response:
```json
{
  "data": [
    {
      "id": "key_abc123",
      "name": "Production Server",
      "scopes": ["triggers:read"],
      "created_at": "2024-01-15T10:00:00Z",
      "last_used_at": "2024-01-15T14:30:00Z"
    }
  ],
  "total": 1
}
```

Note: The actual key value is never returned after creation.

## Rotating API Keys

Rotate keys regularly to maintain security:

```bash
curl -X POST https://api.agentauri.ai/api/v1/api-keys/key_abc123/rotate \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

Response:
```json
{
  "id": "key_abc123",
  "key": "sk_test_xyz789abc123def456ghi789jkl012mno",
  "name": "Production Server",
  "rotated_at": "2024-01-16T10:00:00Z"
}
```

The old key is immediately invalidated.

## Revoking API Keys

```bash
curl -X DELETE https://api.agentauri.ai/api/v1/api-keys/key_abc123 \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Security

### Hashing

API keys are hashed using **Argon2id** before storage. We never store plaintext keys.

### Audit Logging

All API key operations are logged:

- Key creation (with scopes)
- Key usage (endpoint, IP, timestamp)
- Key rotation
- Key revocation

### Best Practices

1. **Use separate keys per environment** - Don't share keys between dev/staging/production
2. **Scope minimally** - Only grant permissions the integration needs
3. **Rotate quarterly** - Or immediately if compromised
4. **Monitor usage** - Check `last_used_at` for inactive keys
5. **Use environment variables** - Never hardcode keys

## Example: Node.js Integration

```javascript
const axios = require('axios');

const client = axios.create({
  baseURL: 'https://api.agentauri.ai/api/v1',
  headers: {
    'X-API-Key': process.env.AGENTAURI_API_KEY
  }
});

// List triggers
const triggers = await client.get('/triggers');
console.log(triggers.data);
```

## Example: Python Integration

```python
import os
import requests

API_KEY = os.environ['AGENTAURI_API_KEY']
BASE_URL = 'https://api.agentauri.ai/api/v1'

headers = {'X-API-Key': API_KEY}

# List triggers
response = requests.get(f'{BASE_URL}/triggers', headers=headers)
triggers = response.json()
```
