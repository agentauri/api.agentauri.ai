---
title: Authentication
description: Learn about AgentAuri's multi-layer authentication system
sidebar:
  order: 2
---

AgentAuri uses a multi-layer authentication system to secure your data and provide flexible access control.

## Authentication Layers

| Layer | Method | Use Case |
|-------|--------|----------|
| Layer 0 | Anonymous | Public endpoints, rate limited by IP |
| Layer 1 | API Key | Server-to-server integration |
| Layer 2 | JWT Token | User sessions, full access |

## JWT Authentication

JWT (JSON Web Token) authentication is the primary method for user sessions.

### Obtaining a Token

**Register a new account:**

```bash
curl -X POST https://api.agentauri.ai/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "myuser",
    "email": "user@example.com",
    "password": "secure-password-123"
  }'
```

**Login to get a token:**

```bash
curl -X POST https://api.agentauri.ai/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "myuser",
    "password": "secure-password-123"
  }'
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-16T12:00:00Z"
}
```

### Using JWT Tokens

Include the token in the `Authorization` header:

```bash
curl https://api.agentauri.ai/api/v1/organizations \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

### Token Expiration

- Tokens expire after **1 hour** by default
- Refresh tokens before expiration to maintain sessions
- Expired tokens return `401 Unauthorized`

## OAuth Authentication

AgentAuri supports OAuth 2.0 with Google and GitHub.

### Google OAuth

```
GET https://api.agentauri.ai/api/v1/auth/google
```

Redirects to Google's OAuth consent screen. After authorization, redirects back with a JWT token.

### GitHub OAuth

```
GET https://api.agentauri.ai/api/v1/auth/github
```

Redirects to GitHub's OAuth authorization page.

## API Key Authentication

For server-to-server integrations, use API keys instead of JWT tokens.

### Creating an API Key

```bash
curl -X POST https://api.agentauri.ai/api/v1/api-keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Server",
    "organization_id": "YOUR_ORG_ID",
    "scopes": ["triggers:read", "triggers:write"]
  }'
```

Response:
```json
{
  "id": "key_abc123",
  "key": "sk_live_xxxxxxxxxxxxxxxxxxxx",
  "name": "Production Server",
  "scopes": ["triggers:read", "triggers:write"]
}
```

:::caution
The API key is only shown once. Store it securely.
:::

### Using API Keys

Include the API key in the `X-API-Key` header:

```bash
curl https://api.agentauri.ai/api/v1/triggers \
  -H "X-API-Key: sk_live_xxxxxxxxxxxxxxxxxxxx"
```

See [API Keys](/getting-started/api-keys) for more details on scopes and rotation.

## Rate Limiting

| Auth Type | Rate Limit |
|-----------|------------|
| Anonymous | 10 requests/minute |
| API Key | 100 requests/minute |
| JWT Token | 100 requests/minute |

Rate limit headers are included in every response:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705410000
```

## Security Best Practices

1. **Never expose tokens in client-side code** - Use server-side proxies
2. **Rotate API keys regularly** - Use the rotation endpoint
3. **Use minimal scopes** - Only request permissions you need
4. **Store secrets securely** - Use environment variables or secret managers
5. **Monitor API key usage** - Check audit logs for suspicious activity
