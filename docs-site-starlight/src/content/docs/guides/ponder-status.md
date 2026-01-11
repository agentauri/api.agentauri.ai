---
title: Ponder Status
description: Monitor blockchain indexer sync status
sidebar:
  order: 5
---

AgentAuri uses [Ponder](https://ponder.sh/) to index blockchain events. These endpoints let you monitor indexer status and event statistics.

## Indexer Status

Check the sync status of blockchain indexers:

```bash
# Note: Ponder status endpoints are PUBLIC (no authentication required)
curl "https://api.agentauri.ai/api/v1/ponder/status"
```

Response:
```json
{
  "chains": [
    {
      "chain": "ethereumSepolia",
      "chain_id": 11155111,
      "current_block": 10003725,
      "is_synced": true
    },
    {
      "chain": "baseSepolia",
      "chain_id": 84532,
      "current_block": 0,
      "is_synced": false
    },
    {
      "chain": "lineaSepolia",
      "chain_id": 59141,
      "current_block": 0,
      "is_synced": false
    }
  ],
  "status": "partial",
  "namespace": "ponder",
  "total_events": 163,
  "last_activity_at": "1767888528"
}
```

### Status Values

| Status | Description |
|--------|-------------|
| `synced` | All chains up to date |
| `partial` | Some chains synced, others pending |
| `syncing` | Catching up to latest blocks |
| `error` | Indexer has encountered an error |

### Response Fields

| Field | Description |
|-------|-------------|
| `chain` | Chain identifier (e.g., "ethereumSepolia") |
| `chain_id` | Network chain ID |
| `current_block` | Last indexed block (0 if not configured) |
| `is_synced` | Whether chain is up to date |
| `status` | Overall indexer status |
| `namespace` | Ponder database namespace |
| `total_events` | Total events indexed |
| `last_activity_at` | Unix timestamp of last activity |

## Event Statistics

Get statistics about indexed events:

```bash
curl "https://api.agentauri.ai/api/v1/ponder/events"
```

Response:
```json
{
  "chains": [
    {
      "chain_id": 11155111,
      "chain_name": "Ethereum Sepolia",
      "events": {
        "Registered": 1250,
        "MetadataSet": 340,
        "URIUpdated": 120,
        "Transfer": 85,
        "NewFeedback": 5670,
        "FeedbackRevoked": 45,
        "ResponseAppended": 890
      },
      "total_events": 20490
    },
    {
      "chain_id": 84532,
      "chain_name": "Base Sepolia",
      "events": {
        "Registered": 890,
        "NewFeedback": 3450
      },
      "total_events": 4340
    }
  ],
  "total_all_chains": 24830,
  "period": {
    "start": "2024-01-01T00:00:00Z",
    "end": "2024-01-15T10:00:00Z"
  }
}
```

### Filter by Chain

```bash
curl "https://api.agentauri.ai/api/v1/ponder/events?chain_id=11155111"
```

### Filter by Time Range

```bash
curl "https://api.agentauri.ai/api/v1/ponder/events?from=2026-01-01&to=2026-01-15"
```

## Monitoring Integration

### Prometheus Metrics

Ponder status is exposed as Prometheus metrics:

```
# Indexer sync status (1=synced, 0=not synced)
agentauri_ponder_synced{chain_id="11155111",chain_name="Ethereum Sepolia"} 1

# Latest indexed block
agentauri_ponder_block_number{chain_id="11155111"} 5234567

# Lag in seconds
agentauri_ponder_lag_seconds{chain_id="11155111"} 12

# Total events indexed
agentauri_ponder_events_total{chain_id="11155111",event_type="Registered"} 1250
```

### Health Check

Include Ponder status in health checks:

```bash
curl "https://api.agentauri.ai/api/v1/health"
```

Response:
```json
{
  "status": "healthy",
  "database": "connected",
  "redis": "connected",
  "ponder": {
    "status": "synced",
    "chains_synced": 3,
    "chains_total": 3
  },
  "version": "1.0.0"
}
```

## Troubleshooting

### Indexer Falling Behind

If `lag_seconds` is high:

1. **Check RPC endpoints** - Verify node connectivity
2. **Review logs** - Look for error patterns
3. **Monitor resources** - Check CPU/memory usage
4. **Contact support** - If issue persists

### Missing Events

If expected events aren't appearing:

1. **Verify chain ID** - Ensure correct network
2. **Check block range** - Confirm block was indexed
3. **Review contract address** - Verify registry address
4. **Check event signature** - Ensure event matches spec

### Status Stuck on "syncing"

If syncing for extended period:

1. **Check initial sync** - First sync takes time
2. **Monitor progress** - Block number should increase
3. **Verify RPC limits** - Some providers have rate limits
4. **Review configuration** - Check start block settings

## Rate Limiting

Ponder status endpoints have special rate limiting:

| Endpoint | Limit |
|----------|-------|
| `/ponder/status` | 60/minute |
| `/ponder/events` | 30/minute |

### Monitoring Token

For monitoring systems, use the monitoring token to bypass rate limits:

```bash
curl "https://api.agentauri.ai/api/v1/ponder/status" \
  -H "X-Monitoring-Token: YOUR_MONITORING_TOKEN"
```

Contact support to obtain a monitoring token.

## Best Practices

1. **Poll periodically** - Check status every 1-5 minutes
2. **Alert on lag** - Set thresholds for acceptable lag
3. **Monitor all chains** - Don't assume all chains sync equally
4. **Cache responses** - Reduce load on monitoring systems
5. **Use monitoring token** - For automated health checks
