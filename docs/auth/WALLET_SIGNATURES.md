# Wallet Signature Authentication

This document describes the wallet signature authentication system (Layer 2) for api.agentauri.ai.

## Overview

Wallet signature authentication enables on-chain agents to authenticate using their Ethereum wallet. This provides:

- Cryptographic proof of agent identity
- Agent → Account linking for billing
- Self-service access to agent-specific data
- Automated feedback processing capability

## EIP-191 Signature Verification

### Message Format

The message to be signed follows EIP-191 personal sign format:

```
api.agentauri.ai Authentication

Agent ID: 42
Chain ID: 84532
Timestamp: 1705312800
Nonce: abc123xyz789
```

### Signature Process

```
1. Agent requests challenge from API
2. API returns message to sign (with nonce)
3. Agent signs message with wallet
4. Agent submits signature to API
5. API verifies signature and issues JWT
```

## Challenge-Response Flow

### Step 1: Request Challenge

```http
POST /api/v1/auth/wallet/challenge
Content-Type: application/json

{
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "agent_id": 42,
  "chain_id": 84532
}
```

**Response**:
```json
{
  "challenge": "api.agentauri.ai Authentication\n\nAgent ID: 42\nChain ID: 84532\nTimestamp: 1705312800\nNonce: abc123xyz789",
  "nonce": "abc123xyz789",
  "expires_at": "2025-01-15T10:05:00Z"
}
```

### Step 2: Sign Message

Using ethers.js:
```javascript
const message = challenge.challenge;
const signature = await signer.signMessage(message);
```

Using viem:
```typescript
const signature = await walletClient.signMessage({
  message: challenge.challenge,
});
```

### Step 3: Submit Signature

```http
POST /api/v1/auth/wallet/verify
Content-Type: application/json

{
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "agent_id": 42,
  "chain_id": 84532,
  "nonce": "abc123xyz789",
  "signature": "0x..."
}
```

**Response** (success):
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "agent_id": 42,
  "chain_id": 84532,
  "linked_account": "org_abc123"
}
```

## Agent → Account Linking

Agents must be linked to an organization account to:
- Inherit payment methods (Credits, Stripe)
- Aggregate rate limits at account level
- Enable centralized billing

### Linking Requirements

1. **Wallet ownership**: Must own the wallet that owns the agent NFT
2. **On-chain verification**: `IdentityRegistry.ownerOf(agentId) == wallet`
3. **One-to-one**: Each agent can only be linked to one account

### Linking Flow

```
1. User authenticates with organization account (JWT)
2. User initiates agent link request
3. API creates challenge for agent owner wallet
4. Agent owner signs challenge
5. API verifies:
   a. Signature is valid
   b. Signer owns the agent (on-chain check)
6. API creates agent_link record
7. Agent can now authenticate via wallet signature
```

### Link Agent Endpoint

```http
POST /api/v1/agents/link
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "agent_id": 42,
  "chain_id": 84532,
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "signature": "0x...",
  "message": "Link agent #42 to organization org_abc123\n\nTimestamp: 1705312800\nNonce: def456"
}
```

**Response**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "agent_id": 42,
  "chain_id": 84532,
  "account_id": "org_abc123",
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "linked_at": "2025-01-15T10:00:00Z",
  "status": "active"
}
```

### List Linked Agents

```http
GET /api/v1/agents/linked
Authorization: Bearer <jwt_token>
```

**Response**:
```json
{
  "agents": [
    {
      "agent_id": 42,
      "chain_id": 84532,
      "wallet_address": "0x1234...",
      "linked_at": "2025-01-15T10:00:00Z",
      "status": "active"
    }
  ]
}
```

### Unlink Agent

```http
DELETE /api/v1/agents/42/link?chain_id=84532
Authorization: Bearer <jwt_token>
```

**Response**:
```json
{
  "agent_id": 42,
  "chain_id": 84532,
  "unlinked_at": "2025-01-20T10:00:00Z"
}
```

## Nonce Management

Nonces prevent replay attacks by ensuring each signature can only be used once.

### Nonce Lifecycle

```
1. Generated: Random 16-byte hex string
2. Issued: Stored with 5-minute expiration
3. Consumed: Marked as used after successful verification
4. Expired: Automatically cleaned up after 24 hours
```

### Database Schema

```sql
CREATE TABLE used_nonces (
    nonce_hash TEXT PRIMARY KEY,      -- SHA-256 of nonce
    agent_id BIGINT,
    wallet_address TEXT,
    used_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL   -- 5 minutes after creation
);

CREATE INDEX idx_used_nonces_expires ON used_nonces(expires_at);
```

### Nonce Validation

```
1. Check nonce exists and not expired
2. Check nonce not already used (hash lookup)
3. If valid:
   a. Mark nonce as used
   b. Proceed with signature verification
4. If invalid/expired:
   a. Reject request
   b. Require new challenge
```

## Database Schema

### Agent Links Table

```sql
CREATE TABLE agent_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id BIGINT NOT NULL,
    chain_id INTEGER NOT NULL,
    account_id TEXT NOT NULL,         -- References organizations.id
    wallet_address TEXT NOT NULL,     -- Checksummed address
    linked_at TIMESTAMPTZ DEFAULT NOW(),
    linked_by_signature TEXT NOT NULL,
    signature_message TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'revoked')),
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (agent_id, chain_id)
);

CREATE INDEX idx_agent_links_agent ON agent_links(agent_id, chain_id)
    WHERE status = 'active';
CREATE INDEX idx_agent_links_account ON agent_links(account_id)
    WHERE status = 'active';
```

### Users Table Enhancement

```sql
ALTER TABLE users ADD COLUMN wallet_address TEXT UNIQUE;
ALTER TABLE users ADD COLUMN auth_method TEXT DEFAULT 'password'
    CHECK (auth_method IN ('password', 'wallet', 'both'));
CREATE INDEX idx_users_wallet ON users(wallet_address)
    WHERE wallet_address IS NOT NULL;
```

## Signature Verification

### Implementation (Rust with alloy)

```rust
use alloy::primitives::{Address, keccak256};
use alloy::signers::Signature;

pub fn verify_eip191_signature(
    message: &str,
    signature: &str,
    expected_address: &str,
) -> Result<bool, AuthError> {
    // Parse the signature
    let sig = Signature::from_str(signature)
        .map_err(|_| AuthError::InvalidSignature)?;

    // Create EIP-191 prefixed message hash
    let prefixed = format!(
        "\x19Ethereum Signed Message:\n{}{}",
        message.len(),
        message
    );
    let hash = keccak256(prefixed.as_bytes());

    // Recover the signer address
    let recovered = sig.recover_address_from_prehash(&hash)
        .map_err(|_| AuthError::RecoveryFailed)?;

    // Compare with expected address
    let expected = Address::from_str(expected_address)
        .map_err(|_| AuthError::InvalidAddress)?;

    Ok(recovered == expected)
}
```

## Request Authentication

For authenticated requests using wallet signature:

```http
GET /api/v1/queries/getMyFeedbacks
X-Agent-Id: 42
X-Chain-Id: 84532
X-Timestamp: 1705312800
X-Nonce: abc123
X-Signature: 0x1234...abcd
```

### Signed Message Format

```
api.agentauri.ai Request

Method: GET
Path: /api/v1/queries/getMyFeedbacks
Agent ID: 42
Chain ID: 84532
Timestamp: 1705312800
Nonce: abc123
```

### Validation Steps

```
1. Extract headers (agent_id, chain_id, timestamp, nonce, signature)
2. Verify timestamp is within 5 minutes of current time
3. Verify nonce is not already used
4. Reconstruct signed message from request
5. Verify signature against reconstructed message
6. Look up agent_link by agent_id and chain_id
7. If linked: apply account rate limits and permissions
8. If not linked: allow basic agent operations only
9. Mark nonce as used
10. Proceed with request
```

## Error Responses

| Status | Code | Description |
|--------|------|-------------|
| 400 | `INVALID_SIGNATURE_FORMAT` | Signature format is invalid |
| 401 | `SIGNATURE_VERIFICATION_FAILED` | Signature doesn't match expected signer |
| 401 | `NONCE_EXPIRED` | Challenge nonce has expired |
| 401 | `NONCE_ALREADY_USED` | Nonce has already been consumed |
| 401 | `TIMESTAMP_EXPIRED` | Request timestamp too old |
| 403 | `AGENT_NOT_LINKED` | Agent not linked to any account |
| 403 | `NOT_AGENT_OWNER` | Wallet doesn't own the agent NFT |

## Security Considerations

1. **Timestamp validation**: Requests must be within 5 minutes
2. **Nonce uniqueness**: Each nonce can only be used once
3. **On-chain verification**: Agent ownership verified on-chain
4. **Address checksumming**: Always use checksummed addresses

## Related Documentation

- [AUTHENTICATION.md](./AUTHENTICATION.md) - Authentication system overview
- [API_KEYS.md](./API_KEYS.md) - API key authentication
- [SECURITY_MODEL.md](./SECURITY_MODEL.md) - Security best practices

---

**Last Updated**: November 24, 2025
