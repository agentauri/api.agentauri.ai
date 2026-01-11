---
title: Wallet Authentication
description: Authenticate with Ethereum wallet signatures (EIP-191)
sidebar:
  order: 6
---

Wallet authentication (Layer 2) allows you to prove ownership of an Ethereum address using cryptographic signatures. This enables linking on-chain agents to your AgentAuri account.

## How It Works

```
1. Request a challenge (nonce)
        │
        ▼
2. Sign the challenge with your wallet
        │
        ▼
3. Submit signature for verification
        │
        ▼
4. Receive JWT token on success
```

## Step 1: Request a Challenge (Nonce)

```bash
curl -X POST "https://api.agentauri.ai/api/v1/auth/nonce" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0x1234567890abcdef1234567890abcdef12345678"
  }'
```

Response:
```json
{
  "nonce": "abc123def456",
  "message": "Sign this message to authenticate with AgentAuri:\n\nNonce: abc123def456\nTimestamp: 2026-01-15T10:00:00Z\nAddress: 0x1234...5678",
  "expires_at": "2026-01-15T10:05:00Z"
}
```

:::note
The nonce expires after 5 minutes. Request a new challenge if it expires.
:::

## Step 2: Sign the Challenge

Sign the challenge message using your Ethereum wallet. The signature must be [EIP-191](https://eips.ethereum.org/EIPS/eip-191) compliant (personal_sign).

### Using ethers.js

```javascript
const { ethers } = require('ethers');

const wallet = new ethers.Wallet(privateKey);
const signature = await wallet.signMessage(challenge);
```

### Using viem

```typescript
import { privateKeyToAccount } from 'viem/accounts';

const account = privateKeyToAccount(privateKey);
const signature = await account.signMessage({ message: challenge });
```

### Using MetaMask

```javascript
const accounts = await ethereum.request({ method: 'eth_requestAccounts' });
const signature = await ethereum.request({
  method: 'personal_sign',
  params: [challenge, accounts[0]]
});
```

## Step 3: Verify the Signature

```bash
curl -X POST "https://api.agentauri.ai/api/v1/auth/wallet" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
    "signature": "0x...",
    "message": "Sign this message to authenticate with AgentAuri:\n\nNonce: abc123def456\nTimestamp: 2026-01-15T10:00:00Z\nAddress: 0x1234...5678"
  }'
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
  "expires_at": "2026-01-15T11:00:00Z",
  "token_type": "Bearer"
}
```

## Agent Linking

Once authenticated with a wallet, you can link on-chain agents to your organization.

### Link an Agent

```bash
curl -X POST "https://api.agentauri.ai/api/v1/agents/link" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "42",
    "chain_id": 11155111,
    "organization_id": "org_abc123"
  }'
```

The system verifies ownership by calling `IdentityRegistry.ownerOf(agentId)` on-chain.

Response:
```json
{
  "id": "link_xyz789",
  "agent_id": "42",
  "chain_id": 11155111,
  "organization_id": "org_abc123",
  "wallet_address": "0x1234...5678",
  "linked_at": "2024-01-15T10:00:00Z"
}
```

### List Linked Agents

```bash
curl "https://api.agentauri.ai/api/v1/agents/linked" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Organization-Id: ORG_ID"
```

Response:
```json
{
  "data": [
    {
      "agent_id": "42",
      "chain_id": 11155111,
      "wallet_address": "0x1234...5678",
      "linked_at": "2024-01-15T10:00:00Z"
    }
  ],
  "total": 1
}
```

### Unlink an Agent

```bash
curl -X DELETE "https://api.agentauri.ai/api/v1/agents/42/link?chain_id=11155111" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Security Features

### Nonce Management

- Each nonce is single-use
- Nonces expire after 5 minutes
- Used nonces are stored to prevent replay attacks
- Background cleanup removes expired nonces

### Signature Verification

AgentAuri uses EIP-191 (personal_sign) verification:

1. Reconstruct the signed message
2. Recover the signer address from the signature
3. Compare with the claimed wallet address
4. Verify the nonce is valid and unused

### Rate Limiting

Wallet authentication endpoints are rate limited:

| Endpoint | Rate Limit |
|----------|------------|
| `/auth/nonce` | 10/minute per IP |
| `/auth/wallet` | 5/minute per IP |

## Error Codes

| Code | Description |
|------|-------------|
| `INVALID_SIGNATURE` | Signature doesn't match address |
| `EXPIRED_NONCE` | Nonce has expired |
| `USED_NONCE` | Nonce already used |
| `INVALID_ADDRESS` | Malformed Ethereum address |
| `AGENT_NOT_OWNED` | Wallet doesn't own the agent |

## Complete Example

```javascript
const { ethers } = require('ethers');
const axios = require('axios');

const API_BASE = 'https://api.agentauri.ai/api/v1';
const wallet = new ethers.Wallet(process.env.PRIVATE_KEY);

async function authenticateWithWallet() {
  // 1. Request nonce
  const nonceRes = await axios.post(`${API_BASE}/auth/nonce`, {
    wallet_address: wallet.address
  });

  const { message } = nonceRes.data;

  // 2. Sign message
  const signature = await wallet.signMessage(message);

  // 3. Verify and get tokens
  const verifyRes = await axios.post(`${API_BASE}/auth/wallet`, {
    wallet_address: wallet.address,
    signature,
    message
  });

  return verifyRes.data.access_token;
}

// Usage
const token = await authenticateWithWallet();
console.log('Authenticated:', token);
```

## Best Practices

1. **Never expose private keys** - Use environment variables or secure key management
2. **Check nonce expiration** - Request a new challenge if needed
3. **Handle signature errors** - Some wallets format signatures differently
4. **Use HTTPS** - Never send signatures over unencrypted connections
5. **Validate on-chain** - Link verification checks actual blockchain state
