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

## Event Types

### Identity Registry

| Event | Description | Key Fields |
|-------|-------------|------------|
| `AgentRegistered` | New agent created | `agent_id`, `owner`, `metadata_uri` |
| `AgentUpdated` | Agent metadata changed | `agent_id`, `metadata_uri` |
| `AgentDeactivated` | Agent marked inactive | `agent_id` |
| `OwnershipTransferred` | Agent ownership changed | `agent_id`, `from`, `to` |

### Reputation Registry

| Event | Description | Key Fields |
|-------|-------------|------------|
| `ReputationUpdated` | Score changed | `agent_id`, `score`, `category` |
| `FeedbackSubmitted` | New feedback received | `agent_id`, `from`, `rating`, `comment` |
| `DisputeOpened` | Reputation dispute | `agent_id`, `disputer`, `reason` |
| `DisputeResolved` | Dispute resolved | `agent_id`, `outcome` |

### Validation Registry

| Event | Description | Key Fields |
|-------|-------------|------------|
| `ValidationRequested` | Validation started | `agent_id`, `validator`, `criteria` |
| `ValidationCompleted` | Validation finished | `agent_id`, `validator`, `passed` |
| `ValidatorRegistered` | New validator added | `validator`, `criteria` |

## Event Structure

Every event contains:

```json
{
  "id": "evt_abc123xyz789",
  "event_type": "AgentRegistered",
  "chain_id": 11155111,
  "block_number": 12345678,
  "block_timestamp": "2024-01-15T10:00:00Z",
  "transaction_hash": "0xabc...def",
  "log_index": 0,
  "contract_address": "0x1234...5678",
  "data": {
    "agent_id": "0xagent...",
    "owner": "0xowner...",
    "metadata_uri": "ipfs://Qm..."
  },
  "indexed_at": "2024-01-15T10:00:05Z"
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
curl "https://api.agentauri.ai/api/v1/events?event_type=AgentRegistered" \
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
| Ethereum Sepolia | 11155111 | Active | ~12s |
| Base Sepolia | 84532 | Active | ~2s |
| Linea Sepolia | 59141 | Active | ~3s |

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
