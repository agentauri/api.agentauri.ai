# Rate Limiting Quick Reference

This is a developer cheat sheet for quick reference while working with the rate limiting system.

**For complete documentation**, see:
- [Rate Limiting User Guide](../auth/RATE_LIMITS_USER_GUIDE.md) - How rate limiting affects API usage
- [Rate Limiting Architecture](./ARCHITECTURE.md) - System design and implementation details

## Components Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Request                              │
│  Headers: Authorization, X-API-Key, X-Forwarded-For          │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
    ┌────────────────────────────┐
    │   IP Extractor             │
    │   (ip_extractor.rs)        │
    │   - X-Forwarded-For        │
    │   - Trusted proxy check    │
    │   - IPv4/IPv6 support      │
    └────────────┬───────────────┘
                 │
                 ▼
    ┌────────────────────────────┐
    │   Auth Extractor           │
    │   (auth_extractor.rs)      │
    │   - Detect layer (0/1/2)   │
    │   - Get organization plan  │
    │   - Determine rate limit   │
    └────────────┬───────────────┘
                 │
                 ▼
    ┌────────────────────────────┐
    │   Rate Limiter             │
    │   (rate_limiter.rs)        │
    │   - Execute Lua script     │
    │   - Atomic check+increment │
    │   - Return result          │
    └────────────┬───────────────┘
                 │
         ┌───────┴────────┐
         │                │
         ▼                ▼
    ALLOWED          REJECTED
    (200 OK)         (429 Too Many Requests)
```

## Authentication Layers

### Layer 0: Anonymous (IP-based)
```
Request → Extract IP → Redis: rl:ip:{ip}:{minute} → Check 10/hour → Allow/Reject
```
- **Limit**: 10 requests/hour
- **Tiers**: 0-1 only
- **Use case**: Public API exploration

### Layer 1: API Key (Organization-based)
```
Request → Validate API Key → Redis: rl:org:{org_id}:{minute} → Check plan limit → Allow/Reject
```
- **Limits**: 50-2000/hour (by plan)
- **Tiers**: 0-3 (all tiers)
- **Format**: `sk_live_xxx` or `sk_test_xxx`

### Layer 2: Wallet Signature (Agent-based)
```
Request → Verify EIP-191 → Get Agent's Org → Redis: rl:org:{org_id}:{minute} → Allow/Reject
```
- **Limits**: Inherits from organization
- **Tiers**: 0-3 + agent operations
- **Authentication**: Challenge-response with signature

## Cost Multipliers

| Tier | Endpoint Example | Cost | Pro Plan (500/hr) |
|------|-----------------|------|-------------------|
| 0 | `/api/v1/feedbacks` | 1x | 500 queries |
| 1 | `/api/v1/reputation/summary` | 2x | 250 queries |
| 2 | `/api/v1/reputation/analysis` | 5x | 100 queries |
| 3 | `/api/v1/reputation/report` | 10x | 50 queries |

## Redis Keys

```
rl:ip:192.168.1.1:1732800600        # Layer 0 (IP)
rl:org:org_abc123:1732800600        # Layer 1/2 (Organization)
rl:agent:42:1732800600              # Layer 2 (Agent-specific ops)
```

**TTL**: 3660 seconds (1 hour + 1 minute buffer)

## Response Headers

All successful requests include:
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 73
X-RateLimit-Reset: 1732804200
X-RateLimit-Window: 3600
```

## Error Response (429)

```json
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

## Code Usage

### Initialize Rate Limiter

```rust
use shared::redis::{create_client, RateLimiter};

// Create Redis connection
let redis = create_client("redis://localhost:6379").await?;

// Create rate limiter
let limiter = RateLimiter::new(redis).await?;
```

### Check Rate Limit

```rust
use shared::RateLimitScope;

// For IP-based (Layer 0)
let result = limiter.check(
    RateLimitScope::Ip("192.168.1.1".to_string()),
    10,  // limit
    1,   // cost (Tier 0)
).await?;

// For Organization-based (Layer 1/2)
let result = limiter.check(
    RateLimitScope::Organization("org_123".to_string()),
    500, // limit (Pro plan)
    5,   // cost (Tier 2 query)
).await?;

if result.allowed {
    println!("Request allowed. Remaining: {}", result.remaining);
} else {
    println!("Rate limited. Retry after: {}", result.retry_after);
}
```

### Extract Auth Context

```rust
use crate::middleware::auth_extractor::extract_auth_context;

let ctx = extract_auth_context(&req, &pool).await;

println!("Layer: {:?}", ctx.layer);
println!("Rate limit: {}/hour", ctx.get_rate_limit());
println!("Allows Tier 3: {}", ctx.allows_tier(3));
```

### Extract IP Address

```rust
use crate::middleware::ip_extractor::extract_ip;

let ip = extract_ip(&req);
println!("Client IP: {}", ip);
```

## Testing

### Unit Tests

```bash
# Test rate limiter
cargo test --lib rate_limiter

# Test auth extractor
cargo test --lib auth_extractor

# Test IP extractor
cargo test --lib ip_extractor

# All tests
cargo test --lib
```

### Integration Test Example

```rust
#[tokio::test]
async fn test_rate_limit_enforcement() {
    let limiter = setup_limiter().await;
    let scope = RateLimitScope::Ip("192.168.1.1".to_string());

    // Allow first 10 requests
    for _ in 0..10 {
        let result = limiter.check(scope.clone(), 10, 1).await.unwrap();
        assert!(result.allowed);
    }

    // 11th request should be rejected
    let result = limiter.check(scope, 10, 1).await.unwrap();
    assert!(!result.allowed);
}
```

## Configuration

### Rate Limit Mode (Shadow vs Enforcing)

```bash
# Production: defaults to "enforcing" (blocks requests)
export ENVIRONMENT=production

# Development: defaults to "shadow" (logs only, allows requests)
# Unset ENVIRONMENT or set to any other value

# Override: explicitly set mode regardless of environment
export RATE_LIMIT_MODE=enforcing   # or "shadow"
```

**Behavior**:
- `shadow`: Log rate limit violations but allow requests through
- `enforcing`: Block requests that exceed limits (return 429)

### Environment Variables

```bash
# Environment (affects rate limit mode)
export ENVIRONMENT=production

# Rate limiting
export RATE_LIMIT_MODE=enforcing         # "shadow" or "enforcing"
export RATE_LIMIT_ENABLED=true
export RATE_LIMIT_FAIL_OPEN=true
export RATE_LIMIT_WINDOW_SECONDS=3600

# Per-plan limits
export RATE_LIMIT_FREE=50
export RATE_LIMIT_STARTER=100
export RATE_LIMIT_PRO=500
export RATE_LIMIT_ENTERPRISE=2000
export RATE_LIMIT_ANONYMOUS=10

# Trusted proxies
export TRUSTED_PROXIES="127.0.0.1,::1,10.0.0.0/8"
```

### Redis Connection

```bash
# Development
export REDIS_HOST=localhost
export REDIS_PORT=6379
export REDIS_PASSWORD=your_password

# Production
export REDIS_URL=redis://:password@redis.example.com:6379
```

### Monitoring Token Bypass

Requests with a valid `X-Monitoring-Token` header bypass rate limiting entirely. This is used by monitoring systems (Grafana, Prometheus, health checkers) to access API endpoints without being subject to rate limits.

**Header Format**:
```http
X-Monitoring-Token: <token-value>
```

**Secret Location**: `agentauri/{env}/monitoring-token` (AWS Secrets Manager)

**Use Cases**:
- Grafana dashboards polling `/api/v1/ponder/status`
- Prometheus scraping metrics endpoints
- External health monitoring services
- Load balancer health checks

**Security Notes**:
- Token should be rotated periodically
- Only share with trusted monitoring systems
- Monitor usage through CloudWatch logs

## Monitoring

### Key Metrics

```
# Request rate
rate_limit_checks_total{scope="ip|org|agent", result="allowed|rejected"}

# Redis latency
rate_limit_redis_duration_seconds{operation="check"}

# Errors
rate_limit_redis_errors_total{error_type="connection|timeout|script"}
```

### Logs

```
# Rate limit check
INFO: Checking rate limit (scope=Organization org_123, limit=500, cost=5)
DEBUG: Rate limit check: ALLOWED (current_usage=245, remaining=250)

# Rate limit exceeded
WARN: Rate limit check: REJECTED (scope=IP 192.168.1.1, usage=10, limit=10, retry_after=3240)

# Redis unavailable
ERROR: Redis error during rate limit check (scope=Organization org_123)
WARN: Redis unavailable, failing open (allowing request)
```

## Troubleshooting

### Rate Limit Not Working

1. Check Redis is running: `redis-cli ping`
2. Verify environment variables: `echo $RATE_LIMIT_ENABLED`
3. Check logs for errors: `grep "rate limit" logs/api-gateway.log`

### High Rejection Rate

1. Check current usage: `limiter.get_current_usage(scope, limit).await`
2. Verify plan limits match user tier
3. Check for query tier cost multipliers

### Redis Connection Issues

1. Verify connection string: `echo $REDIS_URL`
2. Test connection: `redis-cli -h localhost -p 6379 -a password ping`
3. Check Docker: `docker-compose ps redis`

---

**Quick Links**:
- [Full Architecture](./ARCHITECTURE.md)
- [Implementation Summary](./IMPLEMENTATION_SUMMARY.md)
- [API Documentation](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)
