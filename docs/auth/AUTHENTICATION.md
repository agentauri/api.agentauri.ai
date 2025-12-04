# Authentication System

This document provides a comprehensive overview of the 3-layer authentication system for api.agentauri.ai.

## Overview

The authentication system supports multiple client types with different authentication requirements:

| Client Type | Description | Auth Layer | Use Case |
|-------------|-------------|------------|----------|
| On-chain Agents | Registered via ERC-8004 IdentityRegistry | Layer 2 (Wallet) | Query own reputation, receive feedback |
| Developers | Building apps/integrations | Layer 1 (API Key) | Full API access, trigger management |
| Applications | Third-party services | Layer 1 (API Key) + OAuth | Delegated user access |
| Anonymous | No registration required | Layer 0 (None) | Basic queries, x402 micropayments |

## Authentication Layers

### Layer 0: Anonymous Access

No authentication required. Suitable for public data access with micropayment.

**Characteristics**:
- **Auth Method**: None (identified by IP address)
- **Payment Method**: x402 only (crypto micropayments)
- **Rate Limit**: 10 requests/hour per IP address
- **Query Tiers**: 0-1 only (raw and aggregated queries)
- **Use Case**: One-off queries, public exploration, testing

**Request Example**:
```http
GET /api/v1/queries/tier0/getAgentProfile?agentId=42
X-Payment: x402 <payment_proof>
```

**IP Detection and Rate Limiting**:
- Checks `X-Forwarded-For` header if request comes from trusted proxy
- Falls back to direct connection IP address
- Supports both IPv4 and IPv6 addresses
- Rate limit scope: `anon:ip:<ip_address>`

**Rate Limit Response Headers**:
```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 7
X-RateLimit-Reset: 1732800600
X-RateLimit-Window: 3600

{
  "data": { ... }
}
```

**429 Rate Limit Exceeded Example**:
```http
HTTP/1.1 429 Too Many Requests
Retry-After: 2847
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1732803447

{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 2847 seconds. (Limit: 10, Window: 3600s)",
    "retry_after": 2847,
    "limit": 10,
    "window": 3600
  }
}
```

**Query Tier Restrictions**:
- **Allowed**: Tier 0 (1x cost), Tier 1 (2x cost)
- **Blocked**: Tier 2, Tier 3 (requires API Key authentication)
- With 10 requests/hour, you can make:
  - 10 Tier 0 queries (10 × 1 = 10 cost units), OR
  - 5 Tier 1 queries (5 × 2 = 10 cost units), OR
  - Mixed: 6 Tier 0 + 2 Tier 1 = 10 cost units

**Best Practices**:
- Use for testing and exploration only
- Upgrade to API Key (Layer 1) for production use
- Implement client-side caching to reduce calls
- Check `X-RateLimit-Remaining` header before making requests

### Layer 1: API Key Authentication

Account-based authentication for developers and applications.

**Characteristics**:
- **Auth Method**: API Key (`sk_live_xxx` or `sk_test_xxx`)
- **Payment Methods**: Stripe (fiat), x402 (crypto), Credits (prepaid)
- **Rate Limit**: Per-plan (Starter: 100/hr, Pro: 500/hr, Enterprise: 2000/hr)
- **Query Tiers**: 0-3 (all tiers including AI-powered)
- **Use Case**: Applications, integrations, production workloads

**Request Example**:
```http
GET /api/v1/queries/getReputationSummary?agentId=42
Authorization: Bearer sk_live_abc123def456...
```

**Key Format**:
- Prefix: `sk_live_` (production) or `sk_test_` (testing)
- Length: 32 random bytes after prefix (Base64 encoded)
- Storage: Argon2 hash only (original key shown once at creation)

See [API_KEYS.md](./API_KEYS.md) for detailed key management documentation.

### Layer 2: Wallet Signature Verification

On-chain agent authentication via EIP-191 signatures.

**Characteristics**:
- **Auth Method**: Signed message with agent's wallet
- **Payment Methods**: Inherits from linked account
- **Rate Limit**: Inherits from linked account
- **Query Tiers**: 0-3 + agent-specific operations
- **Use Case**: Agent self-queries, automated feedback processing

**Request Example**:
```http
GET /api/v1/queries/getMyFeedbacks
X-Agent-Id: 42
X-Chain-Id: 84532
X-Timestamp: 1705312800
X-Nonce: abc123
X-Signature: 0x1234...abcd
```

**Agent Linking**:
Agents must first be linked to an organization account. This enables:
- Agents to inherit account payment methods
- Rate limit aggregation at account level
- Centralized billing for multiple agents

See [WALLET_SIGNATURES.md](./WALLET_SIGNATURES.md) for signature verification and agent linking details.

## Brute-Force Protection

**Status**: Implemented (December 2, 2025)

The authentication system includes account lockout protection to prevent brute-force password attacks.

### Lockout Policy

| Parameter | Value |
|-----------|-------|
| Threshold | 5 consecutive failed attempts |
| Initial lockout | 15 minutes |
| Maximum lockout | 4 hours |
| Reset condition | Successful login |

### Progressive Lockout Duration

Lockout duration doubles with each occurrence:

| Lockout # | Duration |
|-----------|----------|
| 1st | 15 minutes |
| 2nd | 30 minutes |
| 3rd | 1 hour |
| 4th | 2 hours |
| 5th+ | 4 hours (max) |

### Lockout Response

```http
HTTP/1.1 429 Too Many Requests

{
  "error": {
    "code": "account_locked",
    "message": "Account is temporarily locked. Try again in 847 seconds."
  }
}
```

### Database Fields

Fields added to `users` table:

| Field | Type | Description |
|-------|------|-------------|
| `failed_login_attempts` | INTEGER | Counter, resets on success |
| `locked_until` | TIMESTAMPTZ | Unlock time, NULL = not locked |
| `last_failed_login` | TIMESTAMPTZ | Audit timestamp |

### Behavior

1. **Failed login**: Counter incremented, lockout applied if threshold reached
2. **Successful login**: Counter reset to 0, account unlocked
3. **Locked account**: Returns 429 with seconds remaining
4. **Lock expired**: Account automatically unlocked on next attempt

## Social Authentication

**Status**: Implemented (December 2, 2025)

Users can sign in using Google or GitHub OAuth 2.0.

### Supported Providers

| Provider | Endpoint | Callback |
|----------|----------|----------|
| Google | `/api/v1/auth/google` | `/api/v1/auth/google/callback` |
| GitHub | `/api/v1/auth/github` | `/api/v1/auth/github/callback` |

### Features

- **New user**: Account created automatically with verified email
- **Existing user**: Logs in if email matches existing account
- **Account linking**: Multiple providers can link to one account
- **No password required**: Social-only accounts supported

See [SOCIAL_LOGIN.md](./SOCIAL_LOGIN.md) for complete documentation.

## Authentication Flow

### Layer Precedence

When multiple authentication methods are present, the system checks in order:

```
1. Check for wallet signature (Layer 2)
   ↓ If present and valid → Authenticate as Layer 2

2. Check for API key (Layer 1)
   ↓ If present and valid → Authenticate as Layer 1

3. Fall back to anonymous (Layer 0)
   → IP-based rate limiting, x402 only
```

### Authentication × Payment Matrix

| Layer | Stripe | x402 | Credits | No Payment |
|-------|--------|------|---------|------------|
| 0 (Anonymous) | - | Tier 0-1 | - | - |
| 1 (API Key) | Tier 0-3 | Tier 0-3 | Tier 0-3 | Health only |
| 2 (Wallet) | Inherit | Inherit | Inherit | Agent profile |

### Authentication × Query Tier Matrix

| Layer | Tier 0 (Raw) | Tier 1 (Aggregated) | Tier 2 (Analysis) | Tier 3 (AI) |
|-------|--------------|---------------------|-------------------|-------------|
| 0 (Anonymous) | Yes | Yes | No | No |
| 1 (API Key) | Yes | Yes | Yes | Yes |
| 2 (Wallet) | Yes | Yes | Yes | Yes |

## API Endpoints

### User Authentication

```
POST /api/v1/auth/register    # Create user account
POST /api/v1/auth/login       # Get JWT token
```

### Social Authentication

```
GET  /api/v1/auth/google           # Initiate Google OAuth
GET  /api/v1/auth/google/callback  # Google OAuth callback
GET  /api/v1/auth/github           # Initiate GitHub OAuth
GET  /api/v1/auth/github/callback  # GitHub OAuth callback
```

### API Key Management (Layer 1)

```
POST   /api/v1/api-keys               # Create API key
GET    /api/v1/api-keys               # List organization's keys
GET    /api/v1/api-keys/:id           # Get key details (masked)
DELETE /api/v1/api-keys/:id           # Revoke key
POST   /api/v1/api-keys/:id/rotate    # Rotate key
```

### Wallet Authentication (Layer 2)

```
POST /api/v1/auth/wallet/challenge    # Request signing challenge
POST /api/v1/auth/wallet/verify       # Submit signature, get JWT
```

### Agent Linking

```
POST   /api/v1/agents/link            # Link agent to organization
GET    /api/v1/agents/linked          # List linked agents
DELETE /api/v1/agents/:agent_id/link  # Unlink agent
```

## Implementation Timeline

| Week | Deliverables |
|------|--------------|
| Week 11 | Organizations + API Key Auth (Layer 1) |
| Week 12 | Credits + Wallet Auth (Layer 2) + Agent Linking |
| Week 13 | Rate Limiting + OAuth 2.0 Tables + Layer 0 Foundation |
| Week 19 | x402 Verification → Layer 0 Complete |

## Related Documentation

- [API_KEYS.md](./API_KEYS.md) - API key format, lifecycle, and management
- [SOCIAL_LOGIN.md](./SOCIAL_LOGIN.md) - Google and GitHub OAuth 2.0 integration
- [WALLET_SIGNATURES.md](./WALLET_SIGNATURES.md) - EIP-191 verification and agent linking
- [OAUTH.md](./OAUTH.md) - OAuth 2.0 authorization code flow (for third-party apps)
- [RATE_LIMITING.md](./RATE_LIMITING.md) - Per-tier rate limiting implementation
- [SECURITY_MODEL.md](./SECURITY_MODEL.md) - Threat model and security best practices

---

**Last Updated**: December 2, 2025
