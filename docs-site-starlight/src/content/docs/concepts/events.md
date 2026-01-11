---
title: Events
description: Understanding blockchain events captured by AgentAuri
sidebar:
  order: 3
---

Events are blockchain occurrences captured by AgentAuri's indexers from ERC-8004 registries.

## Event Flow

```
Blockchain Transaction
        │
        ▼
   Smart Contract Emits Event
        │
        ▼
   Ponder Indexer Captures
        │
        ▼
   PostgreSQL Storage
        │
        ▼
   Event Processor Matches Triggers
        │
        ▼
   Actions Executed
```

## Event Types (ERC-8004 v1.0)

### Identity Registry

| Event | Description | Key Fields |
|-------|-------------|------------|
| `Registered` | New agent created | `agentId`, `owner`, `agentURI` |
| `MetadataSet` | Agent metadata key-value set | `agentId`, `metadataKey`, `metadataValue` |
| `URIUpdated` | Agent URI changed | `agentId`, `newURI`, `updatedBy` |
| `Transfer` | Agent ownership transferred (ERC-721) | `tokenId`, `from`, `to` |

### Reputation Registry

| Event | Description | Key Fields |
|-------|-------------|------------|
| `NewFeedback` | Feedback submitted | `agentId`, `clientAddress`, `feedbackIndex`, `score`, `tag1`, `tag2`, `feedbackUri` |
| `FeedbackRevoked` | Feedback revoked by client | `agentId`, `clientAddress`, `feedbackIndex` |
| `ResponseAppended` | Response added to feedback | `agentId`, `clientAddress`, `feedbackIndex`, `responder`, `responseUri` |

### Validation Registry (Not Yet Deployed)

| Event | Description | Key Fields |
|-------|-------------|------------|
| `ValidationRequest` | Validation requested | `agentId`, `validatorAddress`, `requestHash` |
| `ValidationResponse` | Validation response submitted | `agentId`, `validatorAddress`, `response` |

:::note
The Validation Registry contract is not yet deployed on any network. Events listed above reflect the ERC-8004 v1.0 specification.
:::

## Event Structure

Every event contains:

```json
{
  "id": "evt_abc123xyz789",
  "event_type": "Registered",
  "chain_id": 11155111,
  "block_number": 12345678,
  "block_timestamp": "2026-01-15T10:00:00Z",
  "transaction_hash": "0xabc...def",
  "log_index": 0,
  "contract_address": "0x8004A818BFB912233c491871b3d84c89A494BD9e",
  "data": {
    "agentId": "42",
    "owner": "0xowner...",
    "agentURI": "ipfs://Qm..."
  },
  "indexed_at": "2026-01-15T10:00:05Z"
}
```

## Querying Events

### List Recent Events

```bash
curl "https://api.agentauri.ai/api/v1/events?limit=10" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Filter by Event Type

```bash
curl "https://api.agentauri.ai/api/v1/events?event_type=Registered" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Filter by Chain

```bash
curl "https://api.agentauri.ai/api/v1/events?chain_id=11155111" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Filter by Time Range

```bash
curl "https://api.agentauri.ai/api/v1/events?from=2024-01-01T00:00:00Z&to=2024-01-15T23:59:59Z" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Filter by Agent

```bash
curl "https://api.agentauri.ai/api/v1/events?agent_id=0x1234..." \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Supported Networks

| Network | Chain ID | Status | Block Time |
|---------|----------|--------|------------|
| Ethereum Sepolia | 11155111 | **Active** | ~12s |
| Base Sepolia | 84532 | Planned | ~2s |
| Linea Sepolia | 59141 | Planned | ~3s |

:::note
Currently only Ethereum Sepolia has deployed ERC-8004 v1.0 contracts. Other networks are pending deployment.
:::

## Data Retention

- **Raw events**: Stored indefinitely in TimescaleDB hypertable
- **Aggregated metrics**: Computed hourly for analytics
- **Trigger state**: Tracked per-trigger for deduplication

## Event Processing Guarantees

### At-Least-Once Delivery

Events may be processed multiple times in case of failures. Design your actions to be **idempotent**.

### Ordering

Events are processed in block order within each chain. Cross-chain ordering is not guaranteed.

### Latency

| Stage | Typical Latency |
|-------|-----------------|
| Blockchain → Indexer | 1-2 blocks |
| Indexer → Database | < 1 second |
| Database → Trigger Match | < 1 second |
| Action Execution | < 5 seconds |

**Total: 15-30 seconds** from blockchain to action

## Best Practices

1. **Use indexed fields for filtering** - `event_type`, `chain_id`, `agent_id` are indexed
2. **Paginate large queries** - Use `limit` and `offset` parameters
3. **Cache when possible** - Historical events don't change
4. **Handle duplicates** - Use transaction_hash + log_index as unique key
