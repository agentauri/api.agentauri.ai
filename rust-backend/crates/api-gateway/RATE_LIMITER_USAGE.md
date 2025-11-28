# Unified Rate Limiter - Usage Guide

## Quick Start

### 1. Add to main.rs

```rust
use shared::redis::create_client;
use shared::RateLimiter;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // ... existing setup ...

    // Create Redis connection
    let redis = create_client(&config.redis.connection_url())
        .await
        .context("Failed to connect to Redis")?;

    // Create rate limiter
    let rate_limiter = RateLimiter::new(redis)
        .await
        .context("Failed to initialize rate limiter")?;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(middleware::cors())
            .wrap(middleware::DualAuth::new(jwt_secret.clone()))
            .wrap(middleware::QueryTierExtractor::new())
            .wrap(middleware::UnifiedRateLimiter::new(rate_limiter.clone()))
            .app_data(web::Data::new(db_pool.clone()))
            .configure(routes::configure)
    })
    .bind(&server_addr)?
    .run()
    .await?;

    Ok(())
}
```

### 2. Update .env

```bash
# Redis configuration
REDIS_URL=redis://127.0.0.1:6379
# Or with authentication:
REDIS_URL=redis://:password@127.0.0.1:6379
```

### 3. Query Tier Detection

#### Path-based (Recommended)

```
GET /api/v1/queries/tier0/feedbacks      → Tier 0 (1x cost)
GET /api/v1/queries/tier1/summary        → Tier 1 (2x cost)
GET /api/v1/queries/tier2/analysis       → Tier 2 (5x cost)
GET /api/v1/queries/tier3/report         → Tier 3 (10x cost)
```

#### Query parameter

```
GET /api/v1/queries?tier=2               → Tier 2 (5x cost)
GET /api/v1/feedbacks?tier=1             → Tier 1 (2x cost)
```

#### Default

```
GET /api/v1/triggers                     → Tier 0 (1x cost, default)
```

## Rate Limit Rules

### By Authentication Layer

| Layer | Scope | Default Limit | Notes |
|-------|-------|---------------|-------|
| Anonymous (Layer 0) | IP Address | 10/hour | Strict limit, Tier 0-1 only |
| API Key (Layer 1) | Organization | 50-2000/hour | Based on plan |
| Wallet Signature (Layer 2) | Organization | 50-2000/hour | Inherits from org |

### By Subscription Plan

| Plan | Hourly Limit | Tier Access |
|------|--------------|-------------|
| Anonymous | 10 | 0-1 |
| Free | 50 | 0-3 |
| Starter | 100 | 0-3 |
| Pro | 500 | 0-3 |
| Enterprise | 2000 | 0-3 |

### Cost Multipliers

Each request consumes quota based on its tier:

- **Tier 0** (Basic): 1 request = 1 quota unit
- **Tier 1** (Aggregated): 1 request = 2 quota units
- **Tier 2** (Analysis): 1 request = 5 quota units
- **Tier 3** (AI-powered): 1 request = 10 quota units

**Example**: Pro plan (500/hour limit)
- Can make 500 Tier 0 requests
- OR 250 Tier 1 requests
- OR 100 Tier 2 requests
- OR 50 Tier 3 requests
- OR any combination that totals ≤500 quota units

## Response Headers

All responses include rate limit headers:

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 500
X-RateLimit-Remaining: 473
X-RateLimit-Reset: 1732804200
X-RateLimit-Window: 3600
```

## Rate Limit Exceeded (429)

When rate limit is exceeded:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 1847

Rate limit exceeded. Try again in 1847 seconds. (Limit: 500, Window: 3600s)
```

## Handler Access

Access authentication and tier information in your handlers:

```rust
use actix_web::{web, HttpRequest, HttpResponse, Result};
use api_gateway::middleware::{AuthContext, QueryTier};

async fn my_handler(req: HttpRequest) -> Result<HttpResponse> {
    // Get authentication context
    let auth_ctx = req.extensions().get::<AuthContext>()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Missing auth context"))?
        .clone();

    // Get query tier
    let tier = req.extensions().get::<QueryTier>()
        .copied()
        .unwrap_or(QueryTier::Tier0);

    // Log for monitoring
    tracing::info!(
        layer = %auth_ctx.layer.as_str(),
        tier = %tier.as_str(),
        org_id = ?auth_ctx.organization_id,
        "Processing request"
    );

    // Your handler logic
    Ok(HttpResponse::Ok().json(json!({
        "tier": tier.as_str(),
        "cost": tier.cost_multiplier(),
    })))
}
```

## Monitoring

### Structured Logging

The middleware logs all rate limit decisions:

```rust
// Allowed requests (DEBUG level)
debug!(
    scope = "Organization(org_123)",
    current_usage = 47,
    remaining = 453,
    "Rate limit check: ALLOWED"
);

// Rejected requests (WARN level)
warn!(
    scope = "IP(192.168.1.1)",
    current_usage = 11,
    limit = 10,
    retry_after = 1847,
    "Rate limit exceeded"
);

// Redis errors (ERROR level)
error!(
    error = "Connection refused",
    scope = "Organization(org_456)",
    "Rate limiter error - failing open"
);
```

### Prometheus Metrics (Future)

Recommended metrics to add:

```rust
// Rate limit checks
rate_limit_checks_total{status="allowed|rejected|error", layer="anonymous|api_key|wallet"}

// Current usage
rate_limit_usage{scope_type="ip|organization|agent", tier="0|1|2|3"}

// Response times
rate_limit_check_duration_seconds{quantile="0.5|0.9|0.99"}
```

## Graceful Degradation

When Redis is unavailable, the middleware **fails open** (allows requests):

```http
HTTP/1.1 200 OK
X-RateLimit-Status: degraded
```

This ensures service availability even if rate limiting infrastructure fails.

## Testing

### Unit Tests

```bash
# Test query tier detection
cargo test --package api-gateway --lib middleware::query_tier

# Test rate limiter middleware
cargo test --package api-gateway --lib middleware::unified_rate_limiter
```

### Integration Testing

1. Start Redis:
   ```bash
   docker-compose up -d redis
   ```

2. Run integration tests:
   ```bash
   cargo test --package api-gateway --test integration -- --ignored
   ```

3. Manual testing:
   ```bash
   # Make requests until rate limited
   for i in {1..15}; do
     curl -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/v1/triggers
   done
   ```

## Troubleshooting

### Rate Limit Not Applied

1. Check middleware order (UnifiedRateLimiter must come after AuthExtractor)
2. Verify Redis connection: `redis-cli ping`
3. Check logs for "Missing AuthContext" errors

### Redis Connection Failed

If you see:
```
Rate limiter error - failing open
```

Check:
1. Redis is running: `docker-compose ps redis`
2. Redis URL is correct in .env
3. Network connectivity: `telnet localhost 6379`

### Wrong Rate Limit Applied

Verify authentication:
```bash
# Check which auth layer is detected
curl -v -H "X-API-Key: sk_live_xxx" http://localhost:3000/api/v1/triggers 2>&1 | grep X-RateLimit
```

### Rate Limit Not Resetting

Rate limits use a 1-hour sliding window. Check:
- Redis keys: `redis-cli KEYS "rl:*"`
- TTL on keys: `redis-cli TTL "rl:org:org_123:1732800600"`
- Server time is correct: `date +%s`

## Performance Tips

1. **Use Organization-scoped Auth**: API Key and Wallet auth have higher limits than Anonymous
2. **Cache Responses**: Implement response caching to reduce API calls
3. **Batch Requests**: Combine multiple queries into a single Tier 2 analysis query
4. **Upgrade Plan**: Consider Pro or Enterprise plans for high-volume usage
5. **Query Optimization**: Use Tier 0-1 queries when possible (lower cost)

## Security Notes

1. **Never expose API keys**: Store in environment variables, never in code
2. **Use HTTPS**: Rate limit headers reveal usage patterns
3. **Monitor for abuse**: Set up alerts for unusual rate limit rejections
4. **Rotate keys regularly**: Especially after potential exposure
5. **Implement IP allowlisting**: For trusted internal services

## Example Routes Configuration

```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // Public endpoints (no rate limiting needed)
            .route("/health", web::get().to(handlers::health_check))

            // Tiered query endpoints
            .service(
                web::scope("/queries")
                    // Tier 0: Basic queries (1x cost)
                    .service(
                        web::scope("/tier0")
                            .route("/feedbacks", web::get().to(handlers::get_feedbacks))
                            .route("/validations", web::get().to(handlers::get_validations))
                    )
                    // Tier 1: Aggregated queries (2x cost)
                    .service(
                        web::scope("/tier1")
                            .route("/summary", web::get().to(handlers::get_summary))
                            .route("/trends", web::get().to(handlers::get_trends))
                    )
                    // Tier 2: Analysis queries (5x cost)
                    .service(
                        web::scope("/tier2")
                            .route("/analysis", web::get().to(handlers::get_analysis))
                            .route("/compare", web::get().to(handlers::compare_agents))
                    )
                    // Tier 3: AI-powered queries (10x cost)
                    .service(
                        web::scope("/tier3")
                            .route("/report", web::get().to(handlers::generate_report))
                            .route("/dispute", web::get().to(handlers::analyze_dispute))
                    )
            )

            // Other endpoints (default Tier 0)
            .service(
                web::scope("/triggers")
                    .route("", web::post().to(handlers::create_trigger))
                    .route("", web::get().to(handlers::list_triggers))
                    .route("/{id}", web::get().to(handlers::get_trigger))
            )
    );
}
```

## Additional Resources

- **Architecture Documentation**: `/rust-backend/RATE_LIMITER_IMPLEMENTATION.md`
- **Shared Rate Limiter**: `/rust-backend/crates/shared/src/redis/rate_limiter.rs`
- **Auth Context**: `/rust-backend/crates/api-gateway/src/middleware/auth_extractor.rs`
- **Lua Script**: `/rust-backend/crates/shared/src/redis/rate_limit.lua`
