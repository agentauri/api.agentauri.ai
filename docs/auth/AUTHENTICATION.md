# Authentication System

This document provides a comprehensive overview of the 3-layer authentication system for api.8004.dev.

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
- **Rate Limit**: 10 calls/hour per IP
- **Query Tiers**: 0-1 only (raw and aggregated queries)
- **Use Case**: One-off queries, public exploration

**Request Example**:
```http
GET /api/v1/queries/getAgentProfile?agentId=42
X-Payment: x402 <payment_proof>
```

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

### User Authentication (existing)

```
POST /api/v1/auth/register    # Create user account
POST /api/v1/auth/login       # Get JWT token
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
- [WALLET_SIGNATURES.md](./WALLET_SIGNATURES.md) - EIP-191 verification and agent linking
- [RATE_LIMITING.md](./RATE_LIMITING.md) - Per-tier rate limiting implementation
- [SECURITY_MODEL.md](./SECURITY_MODEL.md) - Threat model and security best practices

---

**Last Updated**: November 24, 2025
