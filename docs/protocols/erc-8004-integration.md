# ERC-8004 Integration Guide

## Overview

This document describes how the api.agentauri.ai backend integrates with the ERC-8004 standard for on-chain agent economy. It covers the three core registries, event structures, and integration patterns.

> **⚠️ IMPORTANT**: This documentation reflects the **Jan 2026 Specification Update**. See [Breaking Changes](#jan-2026-specification-changes) section for migration details.

## Quick Links (Always Keep in Context)

| Resource | URL |
|----------|-----|
| **ERC-8004 Specification** | https://eips.ethereum.org/EIPS/eip-8004 |
| **Jan 2026 Spec Changes** | https://github.com/erc-8004/erc-8004-contracts/blob/master/SpecsJan26Update.md |
| **Contracts Repository** | https://github.com/erc-8004/erc-8004-contracts |
| **Contract ABIs** | https://github.com/erc-8004/erc-8004-contracts/tree/master/abis |
| **Subgraph Repository** | https://github.com/agent0lab/subgraph |

## ERC-8004 Standard

**Status**: Draft (in peer-review process)
**Created**: August 13, 2025
**Authors**: Marco De Rossi, Davide Crapis, Jordan Ellis, Erik Reppel

The ERC-8004 standard defines three on-chain registries that establish the foundation for the agent economy:

1. **IdentityRegistry**: Who is the agent? (ERC-721 based portable identifiers)
2. **ReputationRegistry**: How is the agent evaluated? (Scores 0-100 with tags)
3. **ValidationRegistry**: How is the agent's work validated? (TEE, ZK proofs, re-execution)

## Contract Addresses

**Source**: https://github.com/erc-8004/erc-8004-contracts

### Testnet Deployments

| Network | Chain ID | Identity Registry | Reputation Registry | Status |
|---------|----------|-------------------|---------------------|--------|
| **Ethereum Sepolia** | 11155111 | `0x8004A818BFB912233c491871b3d84c89A494BD9e` | `0x8004B663056A597Dffe9eCcC1965A193B7388713` | ✅ Live |
| Base Sepolia | 84532 | - | - | ⏳ Pending |
| Linea Sepolia | 59141 | - | - | ⏳ Pending |
| Polygon Amoy | 80002 | - | - | ⏳ Pending |
| Hedera Testnet | 296 | - | - | ⏳ Pending |
| HyperEVM Testnet | 998 | - | - | ⏳ Pending |
| SKALE Base Sepolia | 1351057110 | - | - | ⏳ Pending |

> **Note**: ValidationRegistry addresses are pending the TEE community integration (planned for later 2026).

## Subgraph

**Repository**: https://github.com/agent0lab/subgraph

### Endpoints

| Network | Status | Endpoint |
|---------|--------|----------|
| Ethereum Sepolia | ✅ Live | `https://gateway.thegraph.com/api/subgraphs/id/6wQRC7geo9XYAhckfmfo8kbMRLeWU8KQd3XsJqFKmZLT` |
| Other networks | ⏳ Pending | Contact maintainers to add chains |

### Indexed Contracts
- IdentityRegistry: Agent registration and metadata
- ReputationRegistry: Feedback and reputation tracking
- ValidationRegistry: Agent validation and attestation

---

## Jan 2026 Specification Changes

> **🔴 BREAKING CHANGES** - Review carefully before updating integrations.

### 1. Elimination of Feedback Pre-Authorization

**OLD Model**: Agents signed `feedbackAuth` structures authorizing specific client addresses.

**NEW Model**: Any address can submit feedback via `giveFeedback()` without pre-authorization.

```solidity
// OLD signature
function giveFeedback(uint256 agentId, uint8 score, bytes32 tag1, bytes32 tag2,
                      string fileuri, bytes32 filehash, bytes feedbackAuth);

// NEW signature
function giveFeedback(uint256 agentId, uint8 score, string tag1, string tag2,
                      string endpoint, string feedbackURI, bytes32 feedbackHash);
```

**Why**: Removes friction; spam resistance moved to off-chain aggregation.

### 2. Agent Wallet Address Verification

**OLD**: Optional off-chain endpoint field.

**NEW**: Reserved on-chain metadata key with cryptographic verification:
- Cannot be set via `setMetadata()` or during `register()`
- Updated only through EIP-712 signatures (EOAs) or ERC-1271 (smart contracts)
- Resets to zero address upon token transfer

### 3. Tags Changed from bytes32 to string

All tag fields in events and functions now use `string` type instead of `bytes32`.

### 4. Renamed Fields

| Old Name | New Name |
|----------|----------|
| `tokenURI` | `agentURI` |
| `tokenId` | `agentId` |
| `fileuri` | `feedbackURI` |
| `filehash` | `feedbackHash` |
| `responseUri` | `responseURI` |

### 5. Registration JSON Schema Updates

**Added Fields**:
- `web` and `email` endpoints
- `x402Support: false`
- `active: true`

**Changed**:
- MCP capabilities: object `{}` → array `[]`
- OASF endpoint version: `0.7` → `0.8`

For complete details, see: https://github.com/erc-8004/erc-8004-contracts/blob/master/SpecsJan26Update.md

---

## IdentityRegistry

### Purpose

Establishes agent identity on-chain using ERC-721 NFTs. Each agent is assigned a unique token ID, and metadata is stored via tokenURI following the ERC-721 URIStorage pattern.

### Contract Interface

```solidity
interface IIdentityRegistry {
    // Events
    event Registered(
        uint256 indexed agentId,
        string tokenURI,
        address indexed owner
    );

    event MetadataSet(
        uint256 indexed agentId,
        bytes32 indexed indexedKey,
        string key,
        string value
    );

    // Functions
    function register(string memory tokenURI) external returns (uint256);
    function setMetadata(uint256 agentId, string memory key, string memory value) external;
    function tokenURI(uint256 agentId) external view returns (string memory);
}
```

### Events

#### Registered

Emitted when a new agent is registered.

**Parameters**:
- `agentId` (uint256, indexed): Unique agent identifier (NFT token ID)
- `tokenURI` (string): IPFS/HTTPS URI pointing to registration file
- `owner` (address, indexed): Agent owner address

**Registration File Format**:

```json
{
  "name": "Trading Agent Alpha",
  "version": "1.0.0",
  "description": "Automated trading agent specializing in DeFi arbitrage",
  "capabilities": {
    "a2a": {
      "endpoint": "https://agent.example.com/a2a",
      "protocols": ["jsonrpc-2.0"]
    },
    "mcp": {
      "endpoint": "https://agent.example.com/mcp",
      "tools": [
        {
          "name": "agent.receiveFeedback",
          "description": "Receive reputation feedback",
          "inputSchema": {
            "type": "object",
            "properties": {
              "score": { "type": "integer", "minimum": 0, "maximum": 100 },
              "clientAddress": { "type": "string" },
              "fileContent": { "type": "object" }
            },
            "required": ["score", "clientAddress"]
          }
        }
      ],
      "authentication": {
        "type": "bearer",
        "tokenHeader": "X-Agent-Token"
      }
    },
    "oasf": {
      "version": "0.8.0",
      "skills": ["trading", "defi", "arbitrage"],
      "domains": ["finance", "blockchain"],
      "modules": [
        {
          "name": "arbitrage-scanner",
          "version": "2.1.0",
          "capabilities": ["scan", "execute"]
        }
      ]
    }
  },
  "metadata": {
    "website": "https://tradingagent.example.com",
    "contact": "admin@example.com",
    "license": "MIT"
  }
}
```

**Backend Handling**:
1. Ponder indexer detects `Registered` event
2. Extract agentId, tokenURI, owner
3. Fetch registration file from tokenURI (IPFS/HTTPS)
4. Parse and cache MCP endpoint, A2A endpoint, OASF metadata
5. Store in Event Store for trigger matching
6. Use cached endpoints for MCP action execution

#### MetadataSet

Emitted when agent metadata is updated (key-value pairs).

**Parameters**:
- `agentId` (uint256, indexed): Agent identifier
- `indexedKey` (bytes32, indexed): Hashed key for efficient filtering
- `key` (string): Metadata key (e.g., "wallet", "status")
- `value` (string): Metadata value

**Common Keys**:
- `wallet`: Agent's payment wallet address
- `status`: Agent operational status (active, paused, deprecated)
- `endpoint.mcp`: Updated MCP endpoint
- `endpoint.a2a`: Updated A2A endpoint

**Backend Handling**:
1. Detect `MetadataSet` event
2. If key is `endpoint.*`, invalidate cached endpoint configuration
3. Refetch tokenURI and update cache
4. Store event in Event Store

### Integration Patterns

**Endpoint Discovery**:
```rust
pub async fn resolve_mcp_endpoint(
    agent_id: u64,
    identity_registry: &IdentityRegistry,
    ipfs_client: &IpfsClient,
) -> Result<McpEndpoint> {
    // Check cache first
    if let Some(cached) = ENDPOINT_CACHE.get(&agent_id) {
        return Ok(cached);
    }

    // Fetch tokenURI from contract
    let token_uri = identity_registry.token_uri(agent_id).await?;

    // Fetch registration file
    let registration_file = ipfs_client.fetch(&token_uri).await?;

    // Parse MCP endpoint configuration
    let endpoint = parse_mcp_endpoint(&registration_file)?;

    // Cache for future use
    ENDPOINT_CACHE.set(agent_id, endpoint.clone());

    Ok(endpoint)
}
```

**Cache Invalidation**:
```rust
pub async fn handle_metadata_set_event(event: MetadataSetEvent) {
    if event.key.starts_with("endpoint.") {
        // Invalidate cache for this agent
        ENDPOINT_CACHE.remove(&event.agent_id);

        info!(
            agent_id = event.agent_id,
            key = event.key,
            "Endpoint metadata updated, cache invalidated"
        );
    }
}
```

## ReputationRegistry

### Purpose

Tracks agent performance through client feedback. Clients submit scores (0-100) with optional semantic tags and supporting files.

> **📋 Jan 2026 Update**: Tags are now `string` instead of `bytes32`. Pre-authorization (`feedbackAuth`) has been removed.

### Contract Interface

```solidity
interface IReputationRegistry {
    // Events (Jan 2026 Updated)
    event NewFeedback(
        uint256 indexed agentId,
        address indexed clientAddress,
        uint256 feedbackIndex,
        uint8 score,
        string indexed tag1,      // Changed from bytes32
        string tag2,              // Changed from bytes32
        string endpoint,          // NEW field
        string feedbackURI,       // Renamed from fileuri
        bytes32 feedbackHash      // Renamed from filehash
    );

    event FeedbackRevoked(
        uint256 indexed agentId,
        address indexed clientAddress,
        uint256 feedbackIndex
    );

    event ResponseAppended(
        uint256 indexed agentId,
        address indexed clientAddress,
        uint256 feedbackIndex,
        address responder,
        string responseURI        // Renamed from responseUri
    );

    // Functions (Jan 2026 Updated - NO feedbackAuth required)
    function giveFeedback(
        uint256 agentId,
        uint8 score,
        string memory tag1,       // Changed from bytes32
        string memory tag2,       // Changed from bytes32
        string memory endpoint,   // NEW field
        string memory feedbackURI,
        bytes32 feedbackHash
    ) external returns (uint256);

    function revokeFeedback(uint256 agentId, uint256 feedbackIndex) external;
    function respondToFeedback(uint256 agentId, uint256 feedbackIndex, string memory responseURI) external;

    // Read functions (Jan 2026 Updated)
    function getSummary(uint256 agentId, string memory tag1, string memory tag2)
        external view returns (uint256 count, uint256 totalScore);
    function readFeedback(uint256 agentId, address client, uint256 feedbackIndex)
        external view returns (Feedback memory);
    function readAllFeedback(uint256 agentId, bool includeRevoked)
        external view returns (Feedback[] memory, uint64[] memory feedbackIndexes);
}
```

### Events

#### NewFeedback

Emitted when a client submits feedback for an agent.

**Parameters**:
- `agentId` (uint256, indexed): Agent being evaluated
- `clientAddress` (address, indexed): Client who submitted feedback
- `feedbackIndex` (uint256): Sequential index for this agent's feedback
- `score` (uint8): Quality score 0-100 (0 = worst, 100 = best)
- `tag1` (bytes32): Primary semantic tag (e.g., "trade", "support")
- `tag2` (bytes32): Secondary semantic tag (e.g., "success", "failure")
- `fileuri` (string): IPFS/HTTPS URI to detailed feedback file
- `filehash` (bytes32): Hash of file content for integrity verification

**Feedback File Format**:

```json
{
  "version": "1.0",
  "score": 85,
  "tags": ["trade", "success"],
  "feedback": {
    "summary": "Excellent trade execution with minimal slippage",
    "details": "Agent executed a complex multi-hop arbitrage trade across Uniswap and Curve. Final profit was 2.3% above estimate.",
    "metrics": {
      "execution_time_ms": 4500,
      "slippage_bps": 12,
      "gas_used": 450000
    }
  },
  "context": {
    "transaction_hash": "0x...",
    "block_number": 12345678,
    "chain_id": 84532
  },
  "mcp_reference": {
    "capability": "tools",
    "name": "agent.trade",
    "invocation_id": "inv_abc123"
  },
  "payment_proof": {
    "amount": "0.05",
    "currency": "ETH",
    "transaction": "0x..."
  }
}
```

**Tag Encoding** (bytes32):

Tags are encoded as bytes32 for efficient on-chain storage:

```typescript
// Encode tag string to bytes32
function encodeTag(tag: string): string {
  return ethers.utils.formatBytes32String(tag);
}

// Decode bytes32 to tag string
function decodeTag(bytes32Tag: string): string {
  return ethers.utils.parseBytes32String(bytes32Tag);
}
```

**Common Tags**:
- `trade`, `support`, `analysis`, `execution`
- `success`, `failure`, `partial`, `timeout`
- `quality`, `speed`, `accuracy`, `cost`

**Backend Handling**:
1. Detect `NewFeedback` event
2. Decode tag1 and tag2 from bytes32 to strings
3. Fetch feedback file from fileuri (IPFS/HTTPS)
4. Verify file integrity (compute hash and compare with filehash)
5. Store complete event in Event Store
6. Trigger evaluation:
   - Match against score threshold triggers
   - Update EMA/counter state for stateful triggers
   - Enqueue MCP action if configured

#### FeedbackRevoked

Emitted when a client revokes previously submitted feedback.

**Parameters**:
- `agentId` (uint256, indexed): Agent whose feedback is revoked
- `clientAddress` (address, indexed): Client who revoked feedback
- `feedbackIndex` (uint256): Index of revoked feedback

**Backend Handling**:
1. Detect `FeedbackRevoked` event
2. Store in Event Store (mark original feedback as revoked)
3. If stateful triggers exist, recalculate state (exclude revoked feedback)
4. Optional: Notify agent via MCP that feedback was revoked

#### ResponseAppended

Emitted when an agent (or agent owner) responds to feedback.

**Parameters**:
- `agentId` (uint256, indexed): Agent responding
- `clientAddress` (address, indexed): Client who received response
- `feedbackIndex` (uint256): Feedback being responded to
- `responder` (address): Address that submitted response
- `responseUri` (string): IPFS/HTTPS URI to response content

**Backend Handling**:
1. Detect `ResponseAppended` event
2. Fetch response content from responseUri
3. Store in Event Store
4. Optional: Notify client via configured channel

### Integration Patterns

**Score Threshold Trigger**:

```sql
-- Trigger condition
SELECT * FROM triggers
WHERE chain_id = 84532
  AND registry = 'reputation'
  AND enabled = true
  AND EXISTS (
    SELECT 1 FROM trigger_conditions
    WHERE trigger_id = triggers.id
      AND condition_type = 'score_threshold'
      AND field = 'score'
      AND operator = '<'
      AND value = '60'
  );
```

**Exponential Moving Average (EMA)**:

```rust
pub fn update_ema_state(
    trigger_id: &str,
    new_score: u8,
    alpha: f64, // Smoothing factor (0.0 - 1.0)
    pool: &PgPool,
) -> Result<f64> {
    // Fetch current EMA from trigger_state
    let current_state = sqlx::query!(
        "SELECT state_data FROM trigger_state WHERE trigger_id = $1",
        trigger_id
    )
    .fetch_one(pool)
    .await?;

    let current_ema: f64 = current_state
        .state_data
        .get("ema")
        .and_then(|v| v.as_f64())
        .unwrap_or(new_score as f64);

    // Calculate new EMA: EMA_new = alpha * score + (1 - alpha) * EMA_old
    let new_ema = alpha * (new_score as f64) + (1.0 - alpha) * current_ema;

    // Update state
    sqlx::query!(
        r#"
        INSERT INTO trigger_state (trigger_id, state_data, last_updated)
        VALUES ($1, $2, NOW())
        ON CONFLICT (trigger_id) DO UPDATE
        SET state_data = $2, last_updated = NOW()
        "#,
        trigger_id,
        serde_json::json!({ "ema": new_ema })
    )
    .execute(pool)
    .await?;

    Ok(new_ema)
}
```

**File Verification**:

```rust
pub async fn verify_feedback_file(
    file_uri: &str,
    expected_hash: &str,
    ipfs_client: &IpfsClient,
) -> Result<serde_json::Value> {
    // Fetch file content
    let content = ipfs_client.fetch(file_uri).await?;

    // Compute hash
    let actual_hash = compute_hash(&content);

    // Verify integrity
    if actual_hash != expected_hash {
        return Err(anyhow!("File hash mismatch: expected {}, got {}", expected_hash, actual_hash));
    }

    // Parse JSON
    let feedback: serde_json::Value = serde_json::from_slice(&content)?;

    Ok(feedback)
}
```

## ValidationRegistry

### Purpose

Enables third-party validators to verify agent capabilities, compliance, or performance. Validators submit requests and responses on-chain, creating a trust layer.

### Contract Interface

```solidity
interface IValidationRegistry {
    // Events
    event ValidationRequest(
        address indexed validatorAddress,
        uint256 indexed agentId,
        string requestUri,
        bytes32 requestHash
    );

    event ValidationResponse(
        address indexed validatorAddress,
        uint256 indexed agentId,
        bytes32 requestHash,
        uint8 response,
        string responseUri,
        bytes32 tag
    );

    // Functions
    function requestValidation(
        uint256 agentId,
        string memory requestUri,
        bytes32 requestHash
    ) external;

    function respondToValidation(
        uint256 agentId,
        bytes32 requestHash,
        uint8 response,
        string memory responseUri,
        bytes32 tag
    ) external;
}
```

### Events

#### ValidationRequest

Emitted when a validator initiates validation for an agent.

**Parameters**:
- `validatorAddress` (address, indexed): Validator initiating request
- `agentId` (uint256, indexed): Agent being validated
- `requestUri` (string): IPFS/HTTPS URI to validation request details
- `requestHash` (bytes32): Hash of request content

**Request File Format**:

```json
{
  "version": "1.0",
  "validator": {
    "name": "Security Audit Validator",
    "address": "0x...",
    "certification": "ISO27001"
  },
  "validation_type": "security_audit",
  "scope": {
    "capabilities": ["agent.trade", "agent.analyze"],
    "test_scenarios": [
      "SQL injection resistance",
      "XSS prevention",
      "Rate limiting compliance"
    ]
  },
  "methodology": "OWASP Top 10 testing",
  "timeline": {
    "requested_at": 1735689600,
    "deadline": 1736294400
  }
}
```

**Backend Handling**:
1. Detect `ValidationRequest` event
2. Store in Event Store
3. Optional: Notify agent via MCP that validation was requested
4. Trigger evaluation for "new validation request" triggers

#### ValidationResponse

Emitted when a validator submits validation results.

**Parameters**:
- `validatorAddress` (address, indexed): Validator submitting response
- `agentId` (uint256, indexed): Agent being validated
- `requestHash` (bytes32): Links response to original request
- `response` (uint8): Validation score 0-100
- `responseUri` (string): IPFS/HTTPS URI to detailed validation report
- `tag` (bytes32): Status tag (e.g., "passed", "failed", "pending")

**Response File Format**:

```json
{
  "version": "1.0",
  "request_hash": "0x...",
  "validator": {
    "name": "Security Audit Validator",
    "address": "0x...",
    "signature": "0x..."
  },
  "validation_result": {
    "score": 92,
    "status": "passed",
    "grade": "A"
  },
  "findings": {
    "critical": 0,
    "high": 1,
    "medium": 3,
    "low": 5,
    "info": 12
  },
  "details": "Agent passed comprehensive security testing with one high-severity finding (rate limit bypass) that was subsequently fixed.",
  "recommendations": [
    "Implement stricter rate limiting on trade execution endpoint",
    "Add input validation for all user-supplied parameters"
  ],
  "completed_at": 1736200000,
  "next_audit_recommended": 1767736000
}
```

**Tag Values**:
- `passed`: Validation successful
- `failed`: Validation failed
- `pending`: Partial results, validation ongoing
- `expired`: Validation expired (for time-limited certifications)

**Backend Handling**:
1. Detect `ValidationResponse` event
2. Decode tag from bytes32
3. Fetch response file from responseUri
4. Verify file integrity
5. Store in Event Store
6. Trigger evaluation:
   - Match against validator whitelist triggers
   - Match against response threshold triggers
   - Enqueue MCP action to notify agent

### Integration Patterns

**Validator Whitelist Trigger**:

```rust
pub async fn check_validator_whitelist(
    trigger_id: &str,
    validator_address: &str,
    pool: &PgPool,
) -> Result<bool> {
    let condition = sqlx::query!(
        r#"
        SELECT value FROM trigger_conditions
        WHERE trigger_id = $1
          AND condition_type = 'validator_whitelist'
        "#,
        trigger_id
    )
    .fetch_one(pool)
    .await?;

    // Parse whitelist from JSON array
    let whitelist: Vec<String> = serde_json::from_str(&condition.value)?;

    // Check if validator is in whitelist (case-insensitive)
    Ok(whitelist
        .iter()
        .any(|addr| addr.eq_ignore_ascii_case(validator_address)))
}
```

## Error Handling

### RPC Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum Erc8004Error {
    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Contract not found at address {0}")]
    ContractNotFound(String),

    #[error("Event parsing failed: {0}")]
    EventParsing(String),

    #[error("IPFS fetch failed for {0}: {1}")]
    IpfsFetch(String, String),

    #[error("File hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}
```

### Retry Strategy

- **RPC calls**: 3 retries with exponential backoff (1s, 2s, 4s)
- **IPFS fetches**: 3 retries with exponential backoff (2s, 4s, 8s)
- **Contract calls**: Fail fast on invalid contract address, retry on network errors

## Testing

### Unit Tests

Test event parsing, tag encoding/decoding, and hash verification in isolation.

### Integration Tests

Use local blockchain (Anvil) with deployed ERC-8004 contracts to test end-to-end event flow.

### Testnet Testing

Monitor actual testnet deployments to verify real-world behavior and reorg handling.

## References

### Core Resources (Always Keep in Context)

| Resource | URL | Description |
|----------|-----|-------------|
| **ERC-8004 Specification** | https://eips.ethereum.org/EIPS/eip-8004 | Official EIP document |
| **Jan 2026 Spec Changes** | https://github.com/erc-8004/erc-8004-contracts/blob/master/SpecsJan26Update.md | Breaking changes guide |
| **Contracts Repository** | https://github.com/erc-8004/erc-8004-contracts | Solidity implementations |
| **Contract ABIs** | https://github.com/erc-8004/erc-8004-contracts/tree/master/abis | JSON ABIs for integration |
| **Subgraph** | https://github.com/agent0lab/subgraph | GraphQL indexing layer |

### Related Standards

- **OASF Schema**: https://github.com/agntcy/oasf
- **A2A Protocol**: https://google.github.io/A2A
- **MCP Protocol**: https://modelcontextprotocol.io/docs

### Deployed Contract Addresses (Ethereum Sepolia)

```
IdentityRegistry:   0x8004A818BFB912233c491871b3d84c89A494BD9e
ReputationRegistry: 0x8004B663056A597Dffe9eCcC1965A193B7388713
```

### Subgraph Endpoint (Ethereum Sepolia)

```
https://gateway.thegraph.com/api/subgraphs/id/6wQRC7geo9XYAhckfmfo8kbMRLeWU8KQd3XsJqFKmZLT
```
