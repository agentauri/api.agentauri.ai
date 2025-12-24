---
title: Rate Limiting
description: Understanding AgentAuri's multi-layer rate limiting system
sidebar:
  order: 3
---

AgentAuri implements a comprehensive rate limiting system to ensure fair usage and protect infrastructure.

## Rate Limiting Layers

| Layer | Scope | Limit Source |
|-------|-------|--------------|
| Layer 0 | IP Address | Anonymous access |
| Layer 1 | Organization | API key plan |
| Layer 2 | Wallet | Inherits org limits |

## Layer 0: Anonymous Access

Unauthenticated requests are rate limited by IP address.

| Limit | Value |
|-------|-------|
| Requests per hour | 10 |
| Burst | 3 |

### Example Request

```bash
curl "https://api.agentauri.ai/api/v1/health"
```

### When Exceeded

```json
{
  "error": "rate_limit_exceeded",
  "message": "Too many requests. Please authenticate for higher limits.",
  "retry_after": 3600
}
```

## Layer 1: API Key Authentication

API key limits are based on your organization's plan.

| Plan | Requests/Hour | Burst |
|------|---------------|-------|
| Free | 50 | 10 |
| Starter | 200 | 50 |
| Pro | 1000 | 200 |
| Enterprise | 5000 | 1000 |

### Example Request

```bash
curl "https://api.agentauri.ai/api/v1/triggers" \
  -H "X-API-Key: sk_live_xxxxxxxxxxxxx"
```

## Layer 2: Wallet Authentication

Wallet-authenticated users inherit their organization's rate limits.

## Rate Limit Headers

Every response includes rate limit headers:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 995
X-RateLimit-Reset: 1705320000
X-RateLimit-Window: 3600
```

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests in window |
| `X-RateLimit-Remaining` | Remaining requests |
| `X-RateLimit-Reset` | Unix timestamp when window resets |
| `X-RateLimit-Window` | Window size in seconds |

## Query Tier Costs

API queries consume rate limit quota based on complexity:

| Tier | Description | Cost Multiplier |
|------|-------------|-----------------|
| Tier 0 | Basic queries | 1x |
| Tier 1 | Aggregated data | 2x |
| Tier 2 | Analysis | 5x |
| Tier 3 | AI-powered | 10x |

### Tier 0 Examples

- `GET /api/v1/events` - List events
- `GET /api/v1/triggers` - List triggers
- `GET /api/v1/agents/linked` - List linked agents

### Tier 1 Examples

- `GET /api/v1/agents/{id}/reputation/summary` - Reputation summary
- `GET /api/v1/events/aggregated` - Event aggregations

### Tier 2 Examples

- `GET /api/v1/agents/{id}/analysis` - Client analysis
- `GET /api/v1/agents/{id}/comparison` - Baseline comparison

### Tier 3 Examples

- `POST /api/v1/agents/{id}/report` - AI-generated report
- `POST /api/v1/disputes/{id}/analysis` - AI dispute analysis

## Sliding Window Algorithm

AgentAuri uses a Redis-based sliding window algorithm:

```
Window: 1 hour (3600 seconds)
Buckets: 60 (1-minute granularity)

Request at 10:45:30:
├── Count requests in buckets 09:46-10:45
├── Weight current bucket (10:45) by time fraction
└── Compare total to limit
```

### Benefits

- Smooth rate limiting (no cliff at window boundary)
- Accurate accounting
- Graceful degradation when Redis unavailable

## Fallback Mode

When Redis is unavailable, rate limiting falls back to in-memory limits:

| Mode | Limit |
|------|-------|
| Fallback | 10 requests/minute per IP |

This conservative limit protects the system during outages.

## Handling Rate Limits

### Check Before Requesting

```javascript
const response = await fetch(url, { headers });
const remaining = response.headers.get('X-RateLimit-Remaining');

if (remaining < 10) {
  console.warn('Approaching rate limit');
}
```

### Retry with Backoff

```javascript
async function requestWithRetry(url, options, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    const response = await fetch(url, options);

    if (response.status === 429) {
      const retryAfter = response.headers.get('Retry-After') || 60;
      await sleep(retryAfter * 1000);
      continue;
    }

    return response;
  }
  throw new Error('Max retries exceeded');
}
```

### Queue Requests

```javascript
const queue = [];
const RATE_LIMIT = 100;
const WINDOW = 3600 * 1000; // 1 hour in ms

function scheduleRequest(request) {
  const now = Date.now();
  const windowStart = now - WINDOW;

  // Remove old requests
  while (queue.length && queue[0] < windowStart) {
    queue.shift();
  }

  if (queue.length >= RATE_LIMIT) {
    const waitTime = queue[0] - windowStart;
    return new Promise(resolve => setTimeout(resolve, waitTime))
      .then(() => scheduleRequest(request));
  }

  queue.push(now);
  return request();
}
```

## Monitoring Usage

### Check Current Usage

```bash
curl "https://api.agentauri.ai/api/v1/billing/usage" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Organization-Id: ORG_ID"
```

Response:
```json
{
  "period_start": "2024-01-15T00:00:00Z",
  "requests_used": 450,
  "requests_limit": 1000,
  "tier_breakdown": {
    "tier_0": 300,
    "tier_1": 100,
    "tier_2": 40,
    "tier_3": 10
  }
}
```

## Best Practices

### 1. Cache Responses

```javascript
const cache = new Map();
const CACHE_TTL = 60000; // 1 minute

async function fetchWithCache(url) {
  const cached = cache.get(url);
  if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
    return cached.data;
  }

  const response = await fetch(url);
  const data = await response.json();

  cache.set(url, { data, timestamp: Date.now() });
  return data;
}
```

### 2. Batch Operations

Instead of:
```javascript
// Bad: 10 requests
for (const id of triggerIds) {
  await fetch(`/triggers/${id}`);
}
```

Use:
```javascript
// Good: 1 request
await fetch(`/triggers?ids=${triggerIds.join(',')}`);
```

### 3. Use Webhooks

Instead of polling, configure webhook actions to push data to your system.

### 4. Choose Appropriate Tiers

- Use Tier 0 queries for basic operations
- Reserve Tier 3 for truly necessary AI analysis
- Cache Tier 2+ results aggressively

## Upgrading Limits

To increase your rate limits:

1. **Upgrade Plan** - Higher plans have higher limits
2. **Contact Support** - Enterprise customers can request custom limits
3. **Optimize Usage** - Reduce unnecessary requests

## Error Responses

### 429 Too Many Requests

```json
{
  "error": "rate_limit_exceeded",
  "message": "Rate limit exceeded. Try again later.",
  "limit": 1000,
  "remaining": 0,
  "reset_at": "2024-01-15T11:00:00Z",
  "retry_after": 1800
}
```

### Headers on 429

```
Retry-After: 1800
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1705320000
```
