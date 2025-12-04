# Troubleshooting Guide

**Project**: api.agentauri.ai - AgentAuri Backend Infrastructure
**Last Updated**: January 30, 2025
**Audience**: Developers, DevOps, SREs

This guide provides solutions to common issues encountered during development, deployment, and operation of the ERC-8004 backend.

---

## Table of Contents

1. [Event Processor Issues](#event-processor-issues)
2. [Database Issues](#database-issues)
3. [Redis Issues](#redis-issues)
4. [Authentication Issues](#authentication-issues)
5. [Ponder Indexer Issues](#ponder-indexer-issues)
6. [Circuit Breaker Issues](#circuit-breaker-issues)
7. [Performance Issues](#performance-issues)
8. [Security Issues](#security-issues)

---

## Event Processor Issues

### ❌ Problem: Events not being processed

**Symptoms**:
```
No events processed in the last 5 minutes
NOTIFY listener appears idle
```

**Possible Causes** & **Solutions**:

#### Cause 1: PostgreSQL NOTIFY listener disconnected

**Check**:
```bash
# Check event-processor logs
docker compose logs event-processor | grep "LISTENER"

# Expected: "Listening for PostgreSQL NOTIFY events on channel 'new_event'"
```

**Solution**:
```bash
# Restart event-processor
docker compose restart event-processor

# Verify reconnection
docker compose logs -f event-processor | grep "Listening"
```

**Prevention**: The listener has automatic reconnection logic with exponential backoff (see `listener.rs:193-243`).

#### Cause 2: Database trigger not firing

**Check**:
```sql
-- Connect to database
docker compose exec postgres psql -U postgres -d agentauri_backend

-- Verify trigger exists
SELECT tgname, tgenabled FROM pg_trigger WHERE tgname = 'notify_new_event_trigger';

-- Check trigger function
\df notify_new_event
```

**Solution**:
```sql
-- If trigger is disabled, enable it
ALTER TABLE events ENABLE TRIGGER notify_new_event_trigger;

-- If trigger is missing, apply migration
-- Run from host: cd rust-backend && sqlx migrate run
```

#### Cause 3: Polling fallback batch aborted

**Check**:
```bash
# Search for batch abort in logs
docker compose logs event-processor | grep "POLLING_FALLBACK_BATCH_ABORTED"
```

**Solution**:
The polling fallback aborts after 10 consecutive failures to prevent cascade errors. This is usually a symptom of:
- Database unavailable
- Redis unavailable
- Circuit breakers all open

**Fix the root cause** (see Database/Redis issues below), then restart:
```bash
docker compose restart event-processor
```

---

### ❌ Problem: Silent task failures (events lost)

**Symptoms**:
```
Events inserted into database but not processed
No error logs
Metrics show task_panicked counter increasing
```

**Diagnosis**:
```bash
# Check for task panics
docker compose logs event-processor | grep "TASK_PANIC"

# Check for timeouts
docker compose logs event-processor | grep "EVENT_PROCESSING_TIMEOUT"
```

**Solution**:
This should NOT happen after Phase 1 fixes (Week 15). If it does:

1. **Check bounded concurrency**:
   ```bash
   # Look for "Active tasks" in metrics
   docker compose logs event-processor | grep "active_tasks"

   # If > 100, semaphore is not working
   ```

2. **Restart with bounded concurrency fix**:
   ```bash
   docker compose restart event-processor
   ```

3. **If panics persist**, file a bug report with logs.

---

### ❌ Problem: High queue depth (Redis)

**Symptoms**:
```
QUEUE_HIGH_DEPTH warning in logs
Queue depth > 10,000
Action workers falling behind
```

**Check**:
```bash
# Check queue depth
docker compose exec redis redis-cli LLEN action_jobs

# Check action worker status
docker compose ps action-workers
docker compose logs action-workers | tail -50
```

**Solutions**:

#### Solution 1: Scale action workers
```bash
# Increase worker replicas
docker compose up -d --scale action-workers=3

# Verify scaling
docker compose ps | grep action-workers
```

#### Solution 2: Check worker health
```bash
# Look for errors in action workers
docker compose logs action-workers | grep -E "(ERROR|CRITICAL)"

# Restart unhealthy workers
docker compose restart action-workers
```

#### Solution 3: Temporary backpressure
If queue reaches **CRITICAL_QUEUE_DEPTH** (50,000), the event-processor will **reject new jobs** until queue drains. This is intentional backpressure to prevent Redis OOM.

**Wait for queue to drain**:
```bash
# Monitor queue depth
watch -n 5 'docker compose exec redis redis-cli LLEN action_jobs'

# Once < 10,000, normal operation resumes
```

---

## Database Issues

### ❌ Problem: TLS connection fails

**Symptoms**:
```
ERROR: connection requires a valid SSL connection
FATAL: no pg_hba.conf entry for host
```

**Solutions**:

#### Solution 1: Certificates missing
```bash
# Verify certificates exist
ls -la docker/postgres/certs/

# If missing, regenerate
./scripts/generate-pg-certs.sh

# Restart PostgreSQL
docker compose restart postgres
```

#### Solution 2: Wrong sslmode
```bash
# Check DATABASE_URL in .env
cat .env | grep DATABASE_URL

# Should include: ?sslmode=require&sslrootcert=./docker/postgres/certs/root.crt

# Fix if missing:
# DATABASE_URL=postgresql://postgres:PASSWORD@localhost:5432/agentauri_backend?sslmode=require&sslrootcert=./docker/postgres/certs/root.crt
```

#### Solution 3: Certificate expired
```bash
# Check expiry
openssl x509 -in docker/postgres/certs/server.crt -noout -enddate

# If expired, regenerate
./scripts/generate-pg-certs.sh
docker compose restart postgres
```

---

### ❌ Problem: Connection pool exhausted

**Symptoms**:
```
ERROR: Failed to acquire connection from pool
Connection timeout after 30s
```

**Check**:
```bash
# Check active connections
docker compose exec postgres psql -U postgres -c "SELECT count(*) FROM pg_stat_activity;"

# Check pool configuration
cat .env | grep DB_MAX_CONNECTIONS
```

**Solutions**:

#### Solution 1: Increase pool size
```bash
# Edit .env
DB_MAX_CONNECTIONS=50  # was 20

# Restart services
docker compose restart api-gateway event-processor
```

#### Solution 2: Check for connection leaks
```sql
-- Find long-running queries
SELECT pid, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active' AND now() - query_start > interval '1 minute'
ORDER BY duration DESC;

-- Kill stuck connections if needed
-- SELECT pg_terminate_backend(pid);
```

---

## Redis Issues

### ❌ Problem: Redis connection refused

**Symptoms**:
```
ERROR: Failed to connect to Redis
Connection refused (os error 111)
```

**Check**:
```bash
# Verify Redis is running
docker compose ps redis

# Test connection
docker compose exec redis redis-cli PING
# Expected: PONG
```

**Solutions**:

#### Solution 1: Start Redis
```bash
docker compose up -d redis

# Verify
docker compose logs redis | tail -20
```

#### Solution 2: Check network
```bash
# Verify containers can reach Redis
docker compose exec event-processor ping redis

# If ping fails, recreate network
docker compose down
docker compose up -d
```

---

## Authentication Issues

### ❌ Problem: API Key authentication fails

**Symptoms**:
```
401 Unauthorized
"Invalid or expired API key"
```

**Diagnosis**:
```bash
# Check API key format
# Must be: sk_live_* or sk_test_*

# Check if key exists in database
docker compose exec postgres psql -U postgres -d agentauri_backend -c \
  "SELECT key_prefix, is_active, expires_at FROM api_keys WHERE key_prefix = 'sk_live_ABC';"
```

**Solutions**:

#### Solution 1: Key revoked or expired
```sql
-- Check key status
SELECT id, key_prefix, is_active, expires_at, revoked_at
FROM api_keys
WHERE key_prefix = 'sk_live_YOUR_PREFIX';

-- If revoked, create new key via API
```

#### Solution 2: Timing attack mitigation issue
After Phase 3.5 Week 11, API key auth uses constant-time verification. If you see:
```
CRITICAL: Failed to verify API key due to missing dummy hash
```

**Fix**:
```bash
# Restart api-gateway to reload dummy hash
docker compose restart api-gateway
```

---

### ❌ Problem: Rate limit exceeded

**Symptoms**:
```
429 Too Many Requests
X-RateLimit-Remaining: 0
```

**Check limits**:
```bash
# Check current rate limit tier
curl -H "Authorization: Bearer YOUR_JWT" \
  https://api.agentauri.ai/api/v1/auth/me

# Response includes: "plan": "free"
```

**Solutions**:

#### Solution 1: Wait for reset
```bash
# Check reset time in response headers
curl -I -H "Authorization: Bearer YOUR_JWT" \
  https://api.agentauri.ai/api/v1/triggers

# Look for: X-RateLimit-Reset: 1640000000
```

#### Solution 2: Upgrade plan
```bash
# Upgrade to Pro plan (500 req/hr)
curl -X POST -H "Authorization: Bearer YOUR_JWT" \
  https://api.agentauri.ai/api/v1/billing/upgrade \
  -d '{"plan": "pro"}'
```

---

## Ponder Indexer Issues

### ❌ Problem: Indexer not syncing

**Symptoms**:
```
Latest indexed block: 12345
Current chain block: 99999
Indexer appears stuck
```

**Check**:
```bash
# Check Ponder logs
cd ponder-indexers
pnpm logs

# Check RPC provider status
curl -X POST YOUR_RPC_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

**Solutions**:

#### Solution 1: RPC rate limit exceeded
```bash
# Check for rate limit errors
pnpm logs | grep "rate limit"

# If found, add fallback RPC provider in ponder.config.ts
# Or wait for rate limit reset
```

#### Solution 2: Chain reorganization
```bash
# Reset indexer to last stable checkpoint
pnpm ponder reset

# Restart indexing
pnpm dev
```

---

## Circuit Breaker Issues

### ❌ Problem: All triggers disabled (circuit breaker OPEN)

**Symptoms**:
```
All triggers showing "Circuit breaker OPEN" in logs
No actions being executed
Error: "skipping trigger (fail-fast)"
```

**Diagnosis**:
```bash
# Check circuit breaker states
docker compose exec postgres psql -U postgres -d agentauri_backend -c \
  "SELECT trigger_id, state, failure_count, last_failure_at FROM circuit_breaker_state WHERE state = 'open';"
```

**Solutions**:

#### Solution 1: Wait for half-open transition
Circuit breakers automatically transition to HALF_OPEN after timeout (default: 60 seconds).

```bash
# Monitor state transitions
docker compose logs event-processor | grep "CIRCUIT_BREAKER"

# Wait for: "Circuit breaker transitioned to HALF_OPEN"
```

#### Solution 2: Manual reset (use with caution)
```sql
-- Only if you've fixed the root cause!
UPDATE circuit_breaker_state
SET state = 'closed', failure_count = 0, success_count = 0
WHERE trigger_id = 'your_trigger_id';
```

#### Solution 3: Fix root cause
Check trigger logs for the actual error:
```bash
docker compose logs event-processor | grep "your_trigger_id" | grep "ERROR"
```

Common causes:
- Webhook endpoint down (REST actions)
- Telegram bot token invalid
- MCP server unreachable

---

### ❌ Problem: Circuit breaker state not persisting

**Symptoms**:
```
ERROR: CIRCUIT_BREAKER_PERSIST_FAILED
Circuit breaker state may be inconsistent
```

**Diagnosis**:
```bash
# Check for persistence failures
docker compose logs event-processor | grep "CIRCUIT_BREAKER_PERSIST_FAILED"
```

**Root cause**: Database write failure (usually transient).

**Solution**:
```bash
# Restart event-processor if persistence failures are frequent
docker compose restart event-processor

# If persists, check database health
docker compose exec postgres psql -U postgres -c "SELECT 1;"
```

---

## Performance Issues

### ❌ Problem: Slow event processing (>1s per event)

**Symptoms**:
```
processing_duration_ms > 1000
Event processing slower than event arrival rate
Backlog growing
```

**Diagnosis**:
```sql
-- Check average processing time
SELECT AVG(processing_duration_ms) as avg_ms,
       MAX(processing_duration_ms) as max_ms,
       COUNT(*) as total_events
FROM processed_events
WHERE processed_at > NOW() - INTERVAL '1 hour';
```

**Solutions**:

#### Solution 1: Too many triggers per event
```bash
# Check trigger count
docker compose logs event-processor | grep "TRIGGER_COUNT_EXCEEDED"

# If > 100, triggers are truncated
# Solution: Remove unused triggers or optimize conditions
```

#### Solution 2: Slow database queries
```sql
-- Enable query logging
ALTER SYSTEM SET log_statement = 'all';
ALTER SYSTEM SET log_min_duration_statement = 100; -- log queries > 100ms
SELECT pg_reload_conf();

-- Check slow queries
SELECT query, calls, mean_exec_time, max_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;
```

#### Solution 3: Redis latency
```bash
# Check Redis latency
docker compose exec redis redis-cli --latency

# If > 10ms, consider:
# - Redis persistence settings
# - Network issues
# - Memory pressure
```

---

## Security Issues

### ❌ Problem: Security headers missing (grade < A)

**Check**:
```bash
./scripts/test-security-headers.sh https://api.agentauri.ai

# Or test specific header
curl -I https://api.agentauri.ai/api/v1/health | grep Strict-Transport-Security
```

**Solutions**:

#### Solution 1: HSTS disabled
```bash
# Enable HSTS in production
# Set in environment: ENABLE_HSTS=true

# Verify
docker compose restart api-gateway
curl -I https://api.agentauri.ai/api/v1/health | grep Strict-Transport-Security
```

#### Solution 2: Middleware not applied
```bash
# Verify SecurityHeaders middleware in code
grep "SecurityHeaders" rust-backend/crates/api-gateway/src/main.rs

# Should see: .wrap(SecurityHeaders::for_api())
```

---

## Getting Help

### Logs to collect for bug reports

```bash
# Event processor logs (last 500 lines)
docker compose logs --tail=500 event-processor > event-processor.log

# Database logs
docker compose logs --tail=500 postgres > postgres.log

# Redis logs
docker compose logs redis > redis.log

# Full docker compose logs
docker compose logs > full-system.log
```

### System state snapshot

```bash
# Container status
docker compose ps > containers.txt

# Resource usage
docker stats --no-stream > resources.txt

# Database stats
docker compose exec postgres psql -U postgres -d agentauri_backend -c \
  "SELECT * FROM pg_stat_activity;" > db-connections.txt
```

### Debugging checklist

Before filing an issue:

- [ ] Checked logs for error messages
- [ ] Verified all containers are running (`docker compose ps`)
- [ ] Checked database connectivity
- [ ] Checked Redis connectivity
- [ ] Reviewed recent code changes
- [ ] Attempted restart (`docker compose restart`)
- [ ] Consulted this troubleshooting guide
- [ ] Searched existing GitHub issues

---

## Related Documentation

- **Circuit Breaker Guide**: [CIRCUIT_BREAKER_GUIDE.md](./CIRCUIT_BREAKER_GUIDE.md)
- **Operations Runbook**: [RUNBOOK.md](./RUNBOOK.md)
- **Database Encryption**: [../security/DATABASE_ENCRYPTION.md](../security/DATABASE_ENCRYPTION.md)
- **API Documentation**: [../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)

---

**Last Updated**: January 30, 2025
**Maintainer**: Development Team
**Feedback**: Open an issue on GitHub
