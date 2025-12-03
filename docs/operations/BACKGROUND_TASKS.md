# Background Tasks

**Status**: Implemented (December 2, 2025)

This document describes the background maintenance tasks that run in the API Gateway for system health and cleanup.

## Overview

The API Gateway runs periodic background tasks using Tokio's async runtime. These tasks handle:

- Expired data cleanup
- Audit log retention
- Resource management

All tasks support graceful shutdown via cancellation tokens.

## Active Tasks

### 1. Nonce Cleanup

**Purpose**: Remove expired wallet authentication nonces from `used_nonces` table

| Configuration | Value |
|--------------|-------|
| Default Interval | 1 hour (3600 seconds) |
| Minimum Interval | 5 minutes (300 seconds) |
| Environment Variable | `NONCE_CLEANUP_INTERVAL_SECS` |

**Behavior**:
- Deletes nonces where `expires_at < NOW()`
- Prevents replay attacks by keeping active nonces
- Logs count of deleted records

**SQL**:
```sql
DELETE FROM used_nonces WHERE expires_at < NOW()
```

### 2. OAuth Token Cleanup

**Purpose**: Remove expired OAuth access and refresh tokens

| Configuration | Value |
|--------------|-------|
| Default Interval | 1 hour (3600 seconds) |
| Minimum Interval | 5 minutes (300 seconds) |
| Environment Variable | `OAUTH_TOKEN_CLEANUP_INTERVAL_SECS` |

**Behavior**:
- Deletes tokens past their expiration timestamp
- Handles both access tokens and refresh tokens
- Logs count of deleted records

### 3. Payment Nonce Cleanup

**Purpose**: Remove expired payment idempotency keys from `payment_nonces` table

| Configuration | Value |
|--------------|-------|
| Default Interval | 1 hour (same as nonce cleanup) |
| Buffer Period | 24 hours after expiration |
| Environment Variable | Uses `NONCE_CLEANUP_INTERVAL_SECS` |

**Behavior**:
- Deletes payment nonces expired more than 24 hours ago
- 24-hour buffer allows for late payment confirmations
- Prevents idempotency key replay attacks

**SQL**:
```sql
DELETE FROM payment_nonces
WHERE expires_at < NOW() - INTERVAL '24 hours'
```

### 4. Auth Failures Cleanup

**Purpose**: Remove old authentication failure audit logs

| Configuration | Value |
|--------------|-------|
| Default Retention | 30 days |
| Minimum Retention | 1 day |
| Environment Variable | `AUTH_FAILURES_RETENTION_DAYS` |

**Behavior**:
- Deletes `auth_failures` records older than retention period
- Maintains security audit trail while preventing table bloat
- Preserves recent failures for security analysis

**SQL**:
```sql
DELETE FROM auth_failures
WHERE created_at < NOW() - INTERVAL '30 days'
```

## Configuration

### Environment Variables

```bash
# Cleanup intervals (in seconds)
NONCE_CLEANUP_INTERVAL_SECS=3600        # Default: 1 hour
OAUTH_TOKEN_CLEANUP_INTERVAL_SECS=3600  # Default: 1 hour

# Retention policies
AUTH_FAILURES_RETENTION_DAYS=30         # Default: 30 days
```

### Minimum Intervals

To prevent excessive database load, intervals have enforced minimums:

| Task | Minimum |
|------|---------|
| Nonce cleanup | 5 minutes |
| OAuth token cleanup | 5 minutes |
| Auth failures retention | 1 day |

Values below minimums are automatically raised.

## Startup and Shutdown

### Initialization

Background tasks start automatically when the API Gateway launches:

```rust
// In main.rs
let bg_runner = BackgroundTaskRunner::new(db_pool.clone());
let shutdown_token = bg_runner.start();
```

**Startup Log**:
```
INFO Background tasks started
  nonce_cleanup_interval_secs=3600
  oauth_token_cleanup_interval_secs=3600
  auth_failures_retention_days=30
```

### Graceful Shutdown

Tasks respond to shutdown signals via `CancellationToken`:

```rust
// On SIGTERM/SIGINT
shutdown_token.cancel();
```

**Shutdown Logs**:
```
INFO Shutdown signal received, stopping background tasks...
INFO Nonce cleanup task stopping due to shutdown
INFO OAuth token cleanup task stopping due to shutdown
INFO Payment nonce cleanup task stopping due to shutdown
INFO Auth failures cleanup task stopping due to shutdown
```

## Monitoring

### Log Messages

| Level | Message | Meaning |
|-------|---------|---------|
| DEBUG | Starting nonce cleanup | Task beginning execution |
| INFO | Cleaned up expired nonces: deleted N | N records removed |
| DEBUG | No expired nonces to clean up | Table already clean |
| ERROR | Failed to cleanup expired nonces | Database error |

### Metrics (Future)

Planned Prometheus metrics:
- `background_task_executions_total{task="nonce_cleanup"}`
- `background_task_duration_seconds{task="nonce_cleanup"}`
- `background_task_records_deleted{task="nonce_cleanup"}`

## Manual Cleanup

For immediate cleanup (useful in maintenance windows):

```rust
use api_gateway::background_tasks;

// Run once (returns count of deleted records)
let deleted = background_tasks::cleanup_nonces_once(&pool).await?;
let deleted = background_tasks::cleanup_oauth_tokens_once(&pool).await?;
let deleted = background_tasks::cleanup_payment_nonces_once(&pool).await?;
let deleted = background_tasks::cleanup_auth_failures_once(&pool, 30).await?;
```

## Tables Affected

| Table | Cleanup Task | Retention |
|-------|--------------|-----------|
| `used_nonces` | Nonce cleanup | Until expiration |
| `oauth_tokens` | OAuth token cleanup | Until expiration |
| `payment_nonces` | Payment nonce cleanup | Expiration + 24h |
| `auth_failures` | Auth failures cleanup | 30 days (configurable) |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    API Gateway                          │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │           BackgroundTaskRunner                   │   │
│  │                                                  │   │
│  │  ┌──────────────┐  ┌──────────────────────┐     │   │
│  │  │ Nonce Cleanup │  │ OAuth Token Cleanup  │     │   │
│  │  │   (tokio)     │  │      (tokio)         │     │   │
│  │  └──────┬───────┘  └──────────┬───────────┘     │   │
│  │         │                      │                 │   │
│  │  ┌──────┴───────┐  ┌──────────┴───────────┐     │   │
│  │  │Payment Nonce │  │ Auth Failures Cleanup│     │   │
│  │  │   Cleanup    │  │      (tokio)         │     │   │
│  │  └──────┬───────┘  └──────────┬───────────┘     │   │
│  │         │                      │                 │   │
│  │         ▼                      ▼                 │   │
│  │  ┌─────────────────────────────────────────┐    │   │
│  │  │          CancellationToken               │    │   │
│  │  │     (graceful shutdown coordination)     │    │   │
│  │  └─────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────┘   │
│                          │                              │
└──────────────────────────┼──────────────────────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │  PostgreSQL  │
                    │   Database   │
                    └──────────────┘
```

## Future Tasks

Planned background tasks for future releases:

- [ ] **Session cleanup**: Remove expired user sessions
- [ ] **Rate limit counter cleanup**: Prune old Redis keys
- [ ] **Trigger state pruning**: Remove orphaned trigger states
- [ ] **Analytics aggregation**: Hourly/daily metric rollups
- [ ] **Notification retry**: Retry failed webhook deliveries

## Troubleshooting

### Tasks Not Running

**Symptom**: No cleanup log messages after startup

**Check**:
1. Verify background tasks started in logs
2. Check database connectivity
3. Ensure no interval < minimum (auto-corrected)

### High Database Load

**Symptom**: Cleanup queries impacting performance

**Solution**:
1. Increase interval: `NONCE_CLEANUP_INTERVAL_SECS=7200`
2. Run during off-peak hours (not currently supported)
3. Add database indexes if missing

### Records Not Being Deleted

**Symptom**: Tables growing despite cleanup running

**Check**:
1. Verify expiration timestamps are in the past
2. Check retention configuration
3. Review error logs for query failures

## Related Documentation

- [RUNBOOK.md](./RUNBOOK.md) - Operational procedures
- [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) - Common issues and solutions
- [CIRCUIT_BREAKER_GUIDE.md](./CIRCUIT_BREAKER_GUIDE.md) - Failure handling patterns

---

**Last Updated**: December 2, 2025
