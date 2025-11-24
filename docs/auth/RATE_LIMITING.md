# Rate Limiting

This document describes the rate limiting implementation for api.8004.dev.

## Overview

Rate limiting protects the API from abuse and ensures fair usage across all clients. The system implements:

- Per-tier rate limits (based on authentication layer)
- Per-account rate limits (based on subscription plan)
- Per-IP rate limits (for anonymous access)
- Redis-based sliding window counters

## Rate Limit Tiers

### By Authentication Layer

| Layer | Auth Method | Rate Limit | Scope |
|-------|-------------|------------|-------|
| 0 | Anonymous (IP) | 10/hour | Per IP address |
| 1 | API Key | Per-plan | Per organization |
| 2 | Wallet Signature | Inherit | Per linked account |

### By Subscription Plan (Layer 1)

| Plan | Requests/Hour | Requests/Day | Concurrent Tasks |
|------|---------------|--------------|------------------|
| Free | 50 | 500 | 1 |
| Starter | 100 | 2,000 | 2 |
| Pro | 500 | 10,000 | 10 |
| Enterprise | 2,000 | 50,000 | 50 |

### By Query Tier

Higher-tier queries consume more rate limit capacity:

| Query Tier | Cost Multiplier | Example |
|------------|-----------------|---------|
| Tier 0 (Raw) | 1x | getMyFeedbacks |
| Tier 1 (Aggregated) | 2x | getReputationSummary |
| Tier 2 (Analysis) | 5x | getClientAnalysis |
| Tier 3 (AI) | 10x | getReputationReport |

## Implementation

### Sliding Window Algorithm

We use a sliding window rate limiter for accurate counting:

```
Window: 1 hour (3600 seconds)
Precision: 60 buckets (1-minute granularity)

Current time: 10:45:30
Window start: 09:45:30
Buckets counted: 09:46, 09:47, ..., 10:45 (60 buckets)
```

### Redis Data Structure

```redis
# Rate limit counter key format
rate_limit:{scope}:{identifier}:{window_start}

# Examples
rate_limit:ip:192.168.1.1:1705312800        # IP-based (Layer 0)
rate_limit:org:org_abc123:1705312800        # Organization (Layer 1)
rate_limit:agent:42:84532:1705312800        # Agent (Layer 2)
```

### Lua Script for Atomic Increment

```lua
-- rate_limit.lua
local key = KEYS[1]
local limit = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local cost = tonumber(ARGV[3])

local current = redis.call('GET', key)
if current == false then
    current = 0
else
    current = tonumber(current)
end

if current + cost > limit then
    return {0, current, limit}  -- Denied
end

redis.call('INCRBY', key, cost)
redis.call('EXPIRE', key, window)
return {1, current + cost, limit}  -- Allowed
```

## Response Headers

All API responses include rate limit headers:

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 73
X-RateLimit-Reset: 1705316400
X-RateLimit-Window: 3600
```

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests in window |
| `X-RateLimit-Remaining` | Remaining requests |
| `X-RateLimit-Reset` | Unix timestamp when window resets |
| `X-RateLimit-Window` | Window duration in seconds |

## Rate Limit Exceeded Response

When rate limit is exceeded:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 1847
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1705316400

{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 1847 seconds.",
    "retry_after": 1847,
    "limit": 100,
    "window": 3600,
    "upgrade_url": "https://8004.dev/pricing"
  }
}
```

## Layer 0: IP-Based Rate Limiting

Anonymous access is rate limited by IP address:

```rust
pub async fn check_ip_rate_limit(
    redis: &RedisPool,
    ip: IpAddr,
) -> Result<RateLimitResult, RateLimitError> {
    let key = format!("rate_limit:ip:{}", ip);
    let limit = 10;  // 10 requests per hour
    let window = 3600;
    let cost = 1;

    check_rate_limit(redis, &key, limit, window, cost).await
}
```

### Considerations

- IPv4 and IPv6 handled separately
- CDN/proxy IP headers respected (`X-Forwarded-For`)
- Shared IPs (NAT) may affect multiple users

## Layer 1: Organization Rate Limiting

API key requests count against organization limits:

```rust
pub async fn check_org_rate_limit(
    redis: &RedisPool,
    org_id: &str,
    plan: Plan,
    query_tier: u8,
) -> Result<RateLimitResult, RateLimitError> {
    let key = format!("rate_limit:org:{}", org_id);
    let limit = plan.hourly_limit();
    let window = 3600;
    let cost = query_tier_cost(query_tier);

    check_rate_limit(redis, &key, limit, window, cost).await
}

fn query_tier_cost(tier: u8) -> i32 {
    match tier {
        0 => 1,
        1 => 2,
        2 => 5,
        3 => 10,
        _ => 1,
    }
}
```

## Layer 2: Agent Rate Limiting

Agent requests inherit rate limits from linked account:

```rust
pub async fn check_agent_rate_limit(
    redis: &RedisPool,
    agent_id: u64,
    chain_id: u32,
    linked_org: Option<&str>,
) -> Result<RateLimitResult, RateLimitError> {
    match linked_org {
        Some(org_id) => {
            // Use organization's rate limit
            check_org_rate_limit(redis, org_id, ...).await
        }
        None => {
            // Unlinked agent: very limited access
            let key = format!("rate_limit:agent:{}:{}", agent_id, chain_id);
            check_rate_limit(redis, &key, 10, 3600, 1).await
        }
    }
}
```

## Burst Handling

Allow short bursts while maintaining hourly limits:

```
Hourly limit: 100 requests
Burst limit: 20 requests per minute
Sustained rate: 100/60 = 1.67 requests per minute
```

Implementation:

```rust
pub struct BurstLimiter {
    hourly_limit: i32,
    burst_limit: i32,
    burst_window: i32,  // 60 seconds
}

impl BurstLimiter {
    pub async fn check(
        &self,
        redis: &RedisPool,
        key: &str,
        cost: i32,
    ) -> Result<RateLimitResult, RateLimitError> {
        // Check burst limit first
        let burst_key = format!("{}:burst", key);
        let burst_result = check_rate_limit(
            redis, &burst_key, self.burst_limit, self.burst_window, cost
        ).await?;

        if !burst_result.allowed {
            return Ok(burst_result);
        }

        // Check hourly limit
        check_rate_limit(redis, key, self.hourly_limit, 3600, cost).await
    }
}
```

## Rate Limit Overrides

### Per-Key Override

Individual API keys can have custom limits:

```sql
-- In api_keys table
rate_limit_override INTEGER  -- NULL = use org default
```

### Temporary Bypass

For incidents or migrations:

```rust
pub async fn check_with_bypass(
    redis: &RedisPool,
    org_id: &str,
) -> Result<RateLimitResult, RateLimitError> {
    // Check for bypass flag
    let bypass_key = format!("rate_limit:bypass:{}", org_id);
    if redis.exists(&bypass_key).await? {
        return Ok(RateLimitResult::allowed());
    }

    // Normal rate limit check
    check_org_rate_limit(redis, org_id, ...).await
}
```

## Monitoring & Alerting

### Prometheus Metrics

```rust
// Rate limit checks
counter!("rate_limit_checks_total", "layer" => layer, "result" => result);

// Rate limit exceeded
counter!("rate_limit_exceeded_total", "layer" => layer, "scope" => scope);

// Current usage percentage
gauge!("rate_limit_usage_percent", "org_id" => org_id);
```

### Alerting Rules

```yaml
# Prometheus alert rules
groups:
  - name: rate_limiting
    rules:
      - alert: HighRateLimitHitRate
        expr: rate(rate_limit_exceeded_total[5m]) > 10
        for: 5m
        annotations:
          summary: "High rate of rate limit hits"

      - alert: OrganizationNearLimit
        expr: rate_limit_usage_percent > 90
        for: 10m
        annotations:
          summary: "Organization approaching rate limit"
```

## Graceful Degradation

When Redis is unavailable:

```rust
pub async fn check_rate_limit_with_fallback(
    redis: &RedisPool,
    key: &str,
    limit: i32,
    window: i32,
    cost: i32,
) -> Result<RateLimitResult, RateLimitError> {
    match check_rate_limit(redis, key, limit, window, cost).await {
        Ok(result) => Ok(result),
        Err(RateLimitError::RedisUnavailable) => {
            // Log warning
            warn!("Redis unavailable, allowing request with local tracking");
            // Use local in-memory counter as fallback
            check_local_rate_limit(key, limit, window, cost)
        }
        Err(e) => Err(e),
    }
}
```

## Related Documentation

- [AUTHENTICATION.md](./AUTHENTICATION.md) - Authentication system overview
- [API_KEYS.md](./API_KEYS.md) - API key authentication
- [SECURITY_MODEL.md](./SECURITY_MODEL.md) - Security best practices

---

**Last Updated**: November 24, 2025
