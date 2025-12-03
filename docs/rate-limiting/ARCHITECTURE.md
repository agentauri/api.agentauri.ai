# Rate Limiting Architecture

This document provides technical implementation details for the rate limiting system.

**For user-facing documentation**, see [Rate Limiting User Guide](../auth/RATE_LIMITS_USER_GUIDE.md).

## Overview

The api.8004.dev rate limiting system provides a comprehensive, Redis-based sliding window rate limiter that supports the 3-layer authentication model with query tier cost multipliers.

## Architecture Components

```
┌─────────────────────────────────────────────────────────────────┐
│                    Incoming HTTP Request                         │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────────────────────────┐
│          Auth Extraction Middleware (Layer Detection)           │
│  Precedence: Wallet Sig (L2) → API Key (L1) → IP (L0)          │
│  Output: AuthContext { layer, org_id, user_id, ip, plan }      │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────────────────────────┐
│              Rate Limit Middleware (Check & Increment)           │
│  - Determine scope (IP / Organization / Agent)                  │
│  - Calculate cost multiplier (Tier 0-3: 1x/2x/5x/10x)           │
│  - Execute Redis Lua script (atomic check + increment)          │
│  - Add X-RateLimit-* headers to response                        │
└────────────────┬────────────────────────────────────────────────┘
                 │
         ┌───────┴────────┐
         │                │
         ▼                ▼
    ┌─────────┐      ┌──────────┐
    │ ALLOWED │      │ REJECTED │
    │ 200 OK  │      │ 429 Too  │
    │         │      │ Many Req │
    └─────────┘      └──────────┘
```

## Data Flow

### Layer 0 (Anonymous - IP-based)

```
Request → Extract IP → Redis Key: "rl:ip:{ip}" → Check Limit (10/hour) → Allow/Reject
```

**Redis Key Pattern**: `rl:ip:192.168.1.1`
**Limit**: 10 requests/hour (Tier 0-1 only)
**Cost**: Tier 0 = 1x, Tier 1 = 2x

### Layer 1 (API Key - Organization-based)

```
Request → Validate API Key → Redis Key: "rl:org:{org_id}" → Check Limit (by plan) → Allow/Reject
```

**Redis Key Pattern**: `rl:org:org_abc123`
**Limits**:
- Free: 50/hour
- Starter: 100/hour
- Pro: 500/hour
- Enterprise: 2000/hour

**Cost**: Tier 0 = 1x, Tier 1 = 2x, Tier 2 = 5x, Tier 3 = 10x

### Layer 2 (Wallet Signature - Inherits from Organization)

```
Request → Verify Signature → Get Agent's Org → Redis Key: "rl:org:{org_id}" → Check Limit → Allow/Reject
```

**Redis Key Pattern**: Same as Layer 1 (inherits organization limits)
**Additional Key**: `rl:agent:{agent_id}` (for agent-specific operations)

## Redis Implementation

### Sliding Window Algorithm

We use a **minute-granularity sliding window** with 60 buckets per hour:

```
Hour Window (3600 seconds)
│
├─ Bucket 0  (0-60s)    ─┐
├─ Bucket 1  (60-120s)   │
├─ Bucket 2  (120-180s)  │
├─ ...                   ├─ 60 buckets
├─ Bucket 58             │
└─ Bucket 59 (3540-3600s)┘
```

**Key Structure**:
```
Key: rl:org:org_123:1732800600  (timestamp at minute boundary)
Value: 15  (requests in that minute)
TTL: 3660 seconds (1 hour + 1 minute buffer)
```

### Lua Script (Atomic Operations)

**File**: `rust-backend/crates/shared/src/redis/rate_limit.lua`

The script performs:
1. Calculate current minute bucket
2. Sum all buckets in the past 60 minutes
3. Check if (current_usage + cost) <= limit
4. If allowed: Increment current bucket, return success
5. If rejected: Return error with retry_after

**Arguments**:
- `KEYS[1]`: Base key prefix (e.g., "rl:org:org_123")
- `ARGV[1]`: Limit (e.g., 100)
- `ARGV[2]`: Window size in seconds (3600)
- `ARGV[3]`: Cost multiplier (1, 2, 5, or 10)
- `ARGV[4]`: Current timestamp

**Returns**:
```lua
{allowed, current_usage, limit, reset_at}
```

### Redis Key Naming Convention

| Scope | Key Pattern | Example |
|-------|-------------|---------|
| IP (Layer 0) | `rl:ip:{ip}:{minute_ts}` | `rl:ip:192.168.1.1:1732800600` |
| Organization (Layer 1/2) | `rl:org:{org_id}:{minute_ts}` | `rl:org:org_abc123:1732800600` |
| Agent (Layer 2 ops) | `rl:agent:{agent_id}:{minute_ts}` | `rl:agent:42:1732800600` |
| Auth Failures | `rl:auth:{ip}:{minute_ts}` | `rl:auth:192.168.1.1:1732800600` |

## Query Tier Cost Multipliers

Different endpoint tiers consume different amounts of rate limit quota:

| Tier | Description | Cost Multiplier | Endpoints |
|------|-------------|-----------------|-----------|
| 0 | Raw queries | 1x | `/api/v1/feedbacks`, `/api/v1/validations` |
| 1 | Aggregated | 2x | `/api/v1/reputation/summary`, `/api/v1/reputation/trend` |
| 2 | Analysis | 5x | `/api/v1/reputation/client-analysis`, `/api/v1/reputation/baseline` |
| 3 | AI-powered | 10x | `/api/v1/reputation/report`, `/api/v1/reputation/dispute-analysis` |

**Example**: A Pro plan organization with 500 req/hour limit:
- Can make 500 Tier 0 queries
- OR 250 Tier 1 queries (500 / 2)
- OR 100 Tier 2 queries (500 / 5)
- OR 50 Tier 3 queries (500 / 10)

## Response Headers

All API responses include rate limit information:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 73
X-RateLimit-Reset: 1732804200
X-RateLimit-Window: 3600
```

**Header Descriptions**:
- `X-RateLimit-Limit`: Total requests allowed in the window
- `X-RateLimit-Remaining`: Requests remaining (accounting for cost)
- `X-RateLimit-Reset`: Unix timestamp when the window resets
- `X-RateLimit-Window`: Window size in seconds (always 3600)

## Error Response (429 Too Many Requests)

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 1847
Content-Type: application/json

{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 1847 seconds.",
    "retry_after": 1847,
    "limit": 100,
    "window": 3600
  }
}
```

## Performance Considerations

### Latency Targets

- **Redis round-trip**: <5ms p95
- **Lua script execution**: <1ms p95
- **Total middleware overhead**: <10ms p95

### Optimizations

1. **Connection Pooling**: Redis connection pool (10-20 connections)
2. **Pipelining**: Batch multiple Redis commands when possible
3. **Local Caching**: Cache organization plan lookups for 60 seconds
4. **Graceful Degradation**: Allow requests if Redis is unavailable (with logging)

### Memory Usage

**Per Organization**:
- 60 minute buckets × 8 bytes per integer = 480 bytes/hour
- With 1000 organizations = ~470 KB total

**Auto-Cleanup**:
- TTL on all keys (3660 seconds)
- Redis automatically evicts expired keys

## Security Features

### Timing Attack Mitigation

The system is designed to prevent timing attacks:
- Rate limit check happens BEFORE any database or crypto operations
- Constant-time key verification (handled by existing API key auth)
- No early returns that leak information

### DDoS Protection

1. **Layer 0 (IP-based)**: 10 req/hour prevents anonymous spam
2. **Global rate limiting**: Authentication rate limiter (separate from query rate limiter)
3. **Cost multipliers**: Expensive operations (AI queries) consume more quota

### Audit Logging

Rate limit violations are logged:
- `api_key_audit_log`: Organization-scoped rate limits
- `auth_failures`: Pre-authentication rate limits

## Graceful Degradation

If Redis is unavailable:
1. Log error with `tracing::error!`
2. Allow request to proceed (fail-open)
3. Emit metrics for monitoring
4. Alert operators via monitoring system

**Rationale**: Better to allow requests than block legitimate users during Redis outage.

## Monitoring & Metrics

### Prometheus Metrics

```
# Rate limit checks
rate_limit_checks_total{scope="ip|org|agent", result="allowed|rejected"}

# Redis latency
rate_limit_redis_duration_seconds{operation="check"}

# Redis errors
rate_limit_redis_errors_total{error_type="connection|timeout|script"}

# Cost multipliers
rate_limit_cost_multiplier{tier="0|1|2|3"}
```

### Alerts

1. **High rejection rate**: >10% of requests rejected (per organization)
2. **Redis latency**: p95 >10ms
3. **Redis errors**: >5 errors/min
4. **Graceful degradation**: Redis fail-open active

## Testing Strategy

### Unit Tests

- Sliding window calculation logic
- Cost multiplier application
- Header formatting
- Error responses

### Integration Tests

- Redis Lua script execution
- Multi-bucket sliding window
- TTL expiration
- Concurrent requests

### Load Tests

- 1000 requests/second sustained
- 100 concurrent organizations
- Redis connection pool exhaustion
- Graceful degradation under Redis failure

## Future Enhancements

### Phase 1 (Current)
- [x] IP-based rate limiting (Layer 0)
- [x] Organization-based rate limiting (Layer 1/2)
- [x] Query tier cost multipliers
- [x] Sliding window algorithm

### Phase 2 (Week 14)
- [ ] Per-agent rate limiting (Layer 2 specific)
- [ ] Burst allowance (short-term over-limit)
- [ ] Dynamic rate limits based on usage patterns

### Phase 3 (Week 15+)
- [ ] Redis Cluster support for horizontal scaling
- [ ] Distributed rate limiting across regions
- [ ] ML-based anomaly detection
- [ ] Rate limit marketplace (sell unused quota)

## Configuration

### Rate Limit Mode

**Status**: Updated December 2, 2025

The rate limiter supports two modes:

| Mode | Behavior | Default Environment |
|------|----------|---------------------|
| `shadow` | Log violations, but allow requests through | Development |
| `enforcing` | Block requests that exceed limits | Production |

**Automatic Mode Selection**:
- When `ENVIRONMENT=production`: defaults to `enforcing`
- When `ENVIRONMENT` is not set or any other value: defaults to `shadow`
- `RATE_LIMIT_MODE` environment variable always takes precedence

**Warning**: If `shadow` mode is used in production, a warning is logged:
```
WARN: Rate limiting is in SHADOW mode in PRODUCTION - requests will NOT be blocked
```

### Environment Variables

```bash
# Environment (affects rate limit mode default)
ENVIRONMENT=production                       # Set to "production" for production deployments

# Rate Limiting
RATE_LIMIT_MODE=enforcing                    # "shadow" (log only) or "enforcing" (block)
RATE_LIMIT_ENABLED=true                      # Enable/disable rate limiting
RATE_LIMIT_FAIL_OPEN=true                    # Allow requests if Redis down
RATE_LIMIT_WINDOW_SECONDS=3600               # Window size (default: 1 hour)
RATE_LIMIT_TIER0_COST=1                      # Tier 0 cost multiplier
RATE_LIMIT_TIER1_COST=2                      # Tier 1 cost multiplier
RATE_LIMIT_TIER2_COST=5                      # Tier 2 cost multiplier
RATE_LIMIT_TIER3_COST=10                     # Tier 3 cost multiplier

# Per-Plan Limits (requests per hour)
RATE_LIMIT_FREE=50                           # Free plan limit
RATE_LIMIT_STARTER=100                       # Starter plan limit
RATE_LIMIT_PRO=500                           # Pro plan limit
RATE_LIMIT_ENTERPRISE=2000                   # Enterprise plan limit
RATE_LIMIT_ANONYMOUS=10                      # Anonymous (Layer 0) limit
```

## Code Organization

```
rust-backend/crates/
├── shared/
│   └── src/
│       ├── redis/
│       │   ├── mod.rs                    # Redis connection pool
│       │   ├── rate_limit.lua            # Lua script for atomic ops
│       │   └── rate_limiter.rs           # RateLimiter service
│       └── services/
│           └── rate_limiter.rs           # Business logic
│
└── api-gateway/
    └── src/
        └── middleware/
            ├── auth_extractor.rs         # AuthContext extraction
            ├── ip_extractor.rs           # IP address extraction
            └── rate_limit.rs             # Rate limit middleware

```

## References

- [Redis Rate Limiting Pattern](https://redis.io/docs/manual/patterns/rate-limiter/)
- [Sliding Window Algorithm](https://hechao.li/2018/06/25/Rate-Limiter-Part1/)
- [HTTP 429 Status Code (RFC 6585)](https://tools.ietf.org/html/rfc6585#section-4)
- [X-RateLimit Headers (Draft RFC)](https://datatracker.ietf.org/doc/html/draft-polli-ratelimit-headers-03)
