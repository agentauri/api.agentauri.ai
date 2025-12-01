# Operations Runbook

**Project**: api.8004.dev - ERC-8004 Backend Infrastructure
**Last Updated**: January 30, 2025
**Audience**: DevOps, SREs, On-call Engineers

This runbook provides standard operating procedures for managing the ERC-8004 backend infrastructure in production.

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Startup Procedures](#startup-procedures)
3. [Shutdown Procedures](#shutdown-procedures)
4. [Health Checks](#health-checks)
5. [Monitoring & Metrics](#monitoring--metrics)
6. [Incident Response](#incident-response)
7. [Backup & Restore](#backup--restore)
8. [Rollback Procedures](#rollback-procedures)
9. [Escalation](#escalation)

---

## System Overview

### Components

The system consists of 5 core services running in a Docker Compose stack:

| Service | Container | Port | Dependencies |
|---------|-----------|------|--------------|
| **PostgreSQL** | `postgres` | 5432 | None |
| **Redis** | `redis` | 6379 | None |
| **API Gateway** | `api-gateway` | 8080 | PostgreSQL, Redis |
| **Event Processor** | `event-processor` | - | PostgreSQL, Redis |
| **Action Workers** | `action-workers` | - | Redis, PostgreSQL |
| **Ponder Indexers** | `ponder-indexers` | 42069 | PostgreSQL, RPC nodes |

### Service Dependencies Graph

```
PostgreSQL ─┬─→ API Gateway ──→ HTTP Clients
            │
            ├─→ Event Processor ─→ Redis ─→ Action Workers
            │
            └─→ Ponder Indexers
                    ↑
                RPC Nodes (Alchemy, Infura)
```

### Data Flow

1. **Event Ingestion**: Ponder Indexers → PostgreSQL events table
2. **Event Notification**: PostgreSQL NOTIFY → Event Processor
3. **Trigger Evaluation**: Event Processor → Trigger matching
4. **Action Queueing**: Event Processor → Redis job queue
5. **Action Execution**: Action Workers → Telegram/REST/MCP endpoints

---

## Startup Procedures

### Complete System Startup

**Time Required**: 2-3 minutes
**Run As**: Application owner (non-root)
**Prerequisites**: Docker installed, `.env` configured

#### Step 1: Verify Prerequisites

```bash
# Verify Docker is running
docker ps

# Verify environment file exists
test -f .env && echo "✓ .env exists" || echo "✗ .env missing"

# Verify required secrets
grep -q "DATABASE_URL" .env && echo "✓ DATABASE_URL set" || echo "✗ DATABASE_URL missing"
grep -q "JWT_SECRET" .env && echo "✓ JWT_SECRET set" || echo "✗ JWT_SECRET missing"
```

#### Step 2: Start Infrastructure Services

Start PostgreSQL and Redis first (dependencies for all other services):

```bash
# Start infrastructure
docker compose up -d postgres redis

# Wait for services to be ready (30s max)
timeout 30 bash -c 'until docker compose exec postgres pg_isready -U postgres; do sleep 1; done'
timeout 30 bash -c 'until docker compose exec redis redis-cli PING | grep -q PONG; do sleep 1; done'

# Verify
docker compose ps postgres redis
```

**Expected Output**:
```
NAME                         STATUS          PORTS
api.8004.dev-postgres-1      Up 30 seconds   0.0.0.0:5432->5432/tcp
api.8004.dev-redis-1         Up 30 seconds   0.0.0.0:6379->6379/tcp
```

#### Step 3: Apply Database Migrations

```bash
# Run migrations (if needed)
cd rust-backend
sqlx migrate run
cd ..

# Verify schema version
docker compose exec postgres psql -U postgres -d erc8004_backend -c \
  "SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1;"
```

#### Step 4: Start Application Services

```bash
# Start API Gateway (depends on PostgreSQL + Redis)
docker compose up -d api-gateway

# Wait for API Gateway (10s)
sleep 10

# Verify API Gateway health
curl -f http://localhost:8080/api/v1/health || echo "API Gateway not ready"
```

**Expected Health Response**:
```json
{
  "status": "healthy",
  "timestamp": "2025-01-30T12:00:00Z",
  "database": "connected",
  "redis": "connected"
}
```

#### Step 5: Start Event Processing Pipeline

```bash
# Start Event Processor (depends on PostgreSQL + Redis)
docker compose up -d event-processor

# Start Action Workers (depends on Redis)
docker compose up -d action-workers

# Wait for services to initialize (10s)
sleep 10

# Verify all services running
docker compose ps
```

#### Step 6: Start Ponder Indexers

```bash
# Start Ponder (depends on PostgreSQL)
docker compose up -d ponder-indexers

# Check Ponder logs
docker compose logs ponder-indexers | tail -20
```

**Expected Log Output**:
```
[info] Started Ponder indexer
[info] Indexing Ethereum Sepolia from block 12345
[info] Indexing Base Sepolia from block 67890
```

#### Step 7: Verify Complete System

Run comprehensive health check:

```bash
./scripts/health-check.sh
```

### Service-Specific Startup

#### API Gateway Only

```bash
# Requires: PostgreSQL, Redis running
docker compose up -d api-gateway
docker compose logs -f api-gateway
```

#### Event Processor Only

```bash
# Requires: PostgreSQL, Redis running
docker compose up -d event-processor
docker compose logs -f event-processor
```

#### Ponder Indexers Only

```bash
# Requires: PostgreSQL running, RPC nodes accessible
cd ponder-indexers
pnpm dev
```

---

## Shutdown Procedures

### Graceful Shutdown (Recommended)

**Time Required**: 30-60 seconds
**Data Loss Risk**: None (all events saved to database)

#### Step 1: Stop Accepting New Requests

```bash
# Stop API Gateway first (no new triggers/requests)
docker compose stop api-gateway

# Verify no new connections
docker compose exec postgres psql -U postgres -c \
  "SELECT COUNT(*) FROM pg_stat_activity WHERE application_name = 'api-gateway';"
# Expected: 0
```

#### Step 2: Drain Event Processing Pipeline

```bash
# Stop Event Processor (no new jobs enqueued)
docker compose stop event-processor

# Wait for Action Workers to finish current jobs (30s max)
timeout 30 bash -c 'until [ $(docker compose exec redis redis-cli LLEN action_jobs) -eq 0 ]; do sleep 2; done'

# Check queue depth
docker compose exec redis redis-cli LLEN action_jobs
# Expected: 0

# Stop Action Workers
docker compose stop action-workers
```

#### Step 3: Stop Indexers

```bash
# Stop Ponder Indexers (checkpoint will be saved)
docker compose stop ponder-indexers
```

#### Step 4: Stop Infrastructure

```bash
# Stop Redis (all jobs should be processed)
docker compose stop redis

# Stop PostgreSQL (all data is persisted)
docker compose stop postgres
```

#### Step 5: Verify All Stopped

```bash
docker compose ps
# All services should show "Exited"
```

### Emergency Shutdown

Use this only if system is unresponsive or critical issue detected.

```bash
# Force stop all services immediately
docker compose down --timeout 10

# Verify all stopped
docker compose ps
```

⚠️ **Warning**: Emergency shutdown may result in:
- In-flight events not fully processed (will be recovered by polling fallback)
- Uncompleted action jobs in Redis queue (will be retried on restart)
- Uncommitted database transactions rolled back

---

## Health Checks

### System Health Check Script

Run comprehensive health check:

```bash
./scripts/health-check.sh
```

**Checks performed**:
- Docker containers running
- PostgreSQL connectivity and schema version
- Redis connectivity
- API Gateway HTTP health endpoint
- Event Processor logs (no errors in last 5 minutes)
- Ponder Indexer sync status
- Queue depth (should be < 1000)

### Manual Health Checks

#### 1. PostgreSQL Health

```bash
# Connection test
docker compose exec postgres pg_isready -U postgres

# Active connections
docker compose exec postgres psql -U postgres -c \
  "SELECT COUNT(*) as active_connections FROM pg_stat_activity WHERE state = 'active';"

# Database size
docker compose exec postgres psql -U postgres -d erc8004_backend -c \
  "SELECT pg_size_pretty(pg_database_size('erc8004_backend'));"
```

**Expected Values**:
- Active connections: < 50 (max 100)
- Database size: varies (monitor growth rate)

#### 2. Redis Health

```bash
# Connection test
docker compose exec redis redis-cli PING
# Expected: PONG

# Queue depth
docker compose exec redis redis-cli LLEN action_jobs
# Expected: < 1000 (warning if > 10000)

# Memory usage
docker compose exec redis redis-cli INFO memory | grep used_memory_human
# Expected: < 500MB
```

#### 3. API Gateway Health

```bash
# HTTP health endpoint
curl -f http://localhost:8080/api/v1/health
# Expected: 200 OK with JSON response

# Response time
curl -o /dev/null -s -w "Response time: %{time_total}s\n" http://localhost:8080/api/v1/health
# Expected: < 0.5s
```

#### 4. Event Processor Health

```bash
# Check for recent activity
docker compose logs event-processor --since 5m | grep -c "Event processed successfully"
# Expected: > 0 (if events are flowing)

# Check for errors
docker compose logs event-processor --since 5m | grep -c "ERROR"
# Expected: 0

# Check task metrics
docker compose logs event-processor | grep "Listener metrics" | tail -1
```

**Expected Metrics Output**:
```
Listener metrics: spawned=1234 succeeded=1230 failed=3 panicked=0 timeout=1 active=2
```

#### 5. Action Workers Health

```bash
# Check for recent activity
docker compose logs action-workers --since 5m | grep -c "Action executed successfully"
# Expected: > 0 (if actions are being processed)

# Check for failed actions
docker compose logs action-workers --since 5m | grep -c "Action execution failed"
# Expected: < 5% of total actions
```

#### 6. Ponder Indexers Health

```bash
# Check sync status
docker compose logs ponder-indexers | grep "Indexing.*from block" | tail -3

# Check for errors
docker compose logs ponder-indexers --since 5m | grep -c "ERROR"
# Expected: 0
```

---

## Monitoring & Metrics

### Key Metrics to Monitor

#### System Metrics

| Metric | Alert Threshold | Action |
|--------|----------------|--------|
| CPU usage | > 80% for 5 min | Scale up or optimize |
| Memory usage | > 85% | Investigate memory leaks |
| Disk usage | > 80% | Clean up logs or expand storage |
| Disk I/O wait | > 20% | Optimize queries or upgrade disk |

#### Application Metrics

| Metric | Alert Threshold | Action |
|--------|----------------|--------|
| API Gateway error rate | > 5% | Check logs, restart if needed |
| API Gateway p95 latency | > 500ms | Investigate slow queries |
| Event processing rate | < 10 events/min (if events exist) | Check Event Processor |
| Queue depth | > 10,000 | Scale Action Workers |
| Action failure rate | > 10% | Check external service health |
| Circuit breakers open | Any trigger | Investigate trigger failures |

#### Database Metrics

| Metric | Alert Threshold | Action |
|--------|----------------|--------|
| Connection pool utilization | > 80% | Increase max_connections |
| Query p95 latency | > 100ms | Optimize slow queries |
| Replication lag | > 1 minute | Check replica health |
| Table size growth | > 10GB/day | Review retention policies |

### Prometheus Metrics

The system exports metrics at `/metrics` endpoint (if enabled):

```bash
# Scrape metrics
curl http://localhost:8080/metrics

# Key metrics to monitor
event_processor.events_processed_total
event_processor.trigger_matches_total
event_processor.action_enqueued_total
event_processor.queue_depth
api_gateway.http_requests_total
api_gateway.http_request_duration_seconds
```

### Log Aggregation

Logs are structured JSON and can be aggregated to Loki/CloudWatch:

```bash
# View recent errors across all services
docker compose logs --since 1h | grep '"level":"error"'

# View specific service logs
docker compose logs event-processor --since 1h | jq 'select(.level == "error")'
```

---

## Incident Response

### Incident Severity Levels

| Severity | Definition | Response Time | Escalation |
|----------|-----------|---------------|------------|
| **P0 (Critical)** | System down, no events processed | Immediate | Page on-call |
| **P1 (High)** | Partial outage, >50% error rate | < 15 min | Alert team |
| **P2 (Medium)** | Degraded performance, <50% error rate | < 1 hour | Log incident |
| **P3 (Low)** | Non-critical issue, monitoring alert | < 4 hours | Backlog |

### Common Incidents & Resolution

#### Incident 1: API Gateway Down

**Symptoms**: HTTP 502/503, health check fails

**Diagnosis**:
```bash
# Check container status
docker compose ps api-gateway

# Check logs for crash
docker compose logs api-gateway --tail 100

# Check dependencies
docker compose exec postgres pg_isready
docker compose exec redis redis-cli PING
```

**Resolution**:
```bash
# Restart API Gateway
docker compose restart api-gateway

# If persists, check database connectivity
docker compose exec postgres psql -U postgres -c "SELECT 1;"

# If database issue, see Incident 3
```

#### Incident 2: Events Not Being Processed

**Symptoms**: Event count not increasing, no logs in Event Processor

**Diagnosis**:
```bash
# Check Event Processor status
docker compose ps event-processor

# Check PostgreSQL NOTIFY listener
docker compose logs event-processor | grep "Listening for PostgreSQL NOTIFY"

# Check for errors
docker compose logs event-processor --tail 100
```

**Resolution**:
```bash
# Restart Event Processor
docker compose restart event-processor

# Verify polling fallback is working (backup mechanism)
docker compose logs event-processor | grep "Polling fallback"

# Check recent processed events
docker compose exec postgres psql -U postgres -d erc8004_backend -c \
  "SELECT COUNT(*) FROM processed_events WHERE processed_at > NOW() - INTERVAL '5 minutes';"
```

#### Incident 3: Database Connection Pool Exhausted

**Symptoms**: "Failed to acquire connection from pool", 500 errors

**Diagnosis**:
```bash
# Check active connections
docker compose exec postgres psql -U postgres -c \
  "SELECT COUNT(*) FROM pg_stat_activity WHERE state = 'active';"

# Check long-running queries
docker compose exec postgres psql -U postgres -c \
  "SELECT pid, now() - query_start AS duration, query FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '1 minute' ORDER BY duration DESC LIMIT 10;"
```

**Resolution**:
```bash
# Kill stuck connections (if any)
docker compose exec postgres psql -U postgres -c \
  "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '5 minutes';"

# Increase connection pool (temporary fix)
# Edit .env: DB_MAX_CONNECTIONS=50
docker compose restart api-gateway event-processor

# Permanent fix: Optimize slow queries
```

#### Incident 4: Redis Queue Overflow

**Symptoms**: Queue depth > 50,000, "QUEUE_CRITICAL_DEPTH" errors

**Diagnosis**:
```bash
# Check queue depth
docker compose exec redis redis-cli LLEN action_jobs

# Check Action Workers status
docker compose ps action-workers
docker compose logs action-workers --tail 50
```

**Resolution**:
```bash
# Scale Action Workers
docker compose up -d --scale action-workers=3

# Monitor queue draining
watch -n 5 'docker compose exec redis redis-cli LLEN action_jobs'

# If queue doesn't drain, check worker errors
docker compose logs action-workers | grep "ERROR"
```

#### Incident 5: All Circuit Breakers Open

**Symptoms**: No actions executed, "Circuit breaker OPEN" in logs

**Diagnosis**:
```bash
# Check circuit breaker states
docker compose exec postgres psql -U postgres -d erc8004_backend -c \
  "SELECT trigger_id, state, failure_count, last_failure_at FROM circuit_breaker_state WHERE state = 'open';"

# Check Action Worker errors
docker compose logs action-workers --since 30m | grep "Action execution failed"
```

**Resolution**:
```bash
# Investigate root cause (webhook down, Telegram rate limit, etc.)
# Fix external service issue first

# Wait for auto-recovery (60 seconds to half-open)
# Or manually reset (use with caution):
docker compose exec postgres psql -U postgres -d erc8004_backend -c \
  "UPDATE circuit_breaker_state SET state = 'closed', failure_count = 0 WHERE trigger_id = 'YOUR_TRIGGER_ID';"
```

### Incident Response Workflow

1. **Acknowledge**: Confirm incident received
2. **Assess**: Determine severity (P0-P3)
3. **Communicate**: Update status page, notify stakeholders
4. **Diagnose**: Use runbook procedures above
5. **Resolve**: Apply fix, verify resolution
6. **Document**: Write postmortem (for P0/P1)
7. **Follow-up**: Implement preventive measures

---

## Backup & Restore

### Database Backups

#### Automated Backups

Production databases should have automated daily backups configured via cloud provider (AWS RDS, Render, etc.).

**Verify backup schedule**:
```bash
# AWS RDS example
aws rds describe-db-instances --db-instance-identifier erc8004-prod \
  --query 'DBInstances[0].[BackupRetentionPeriod,PreferredBackupWindow]'
```

#### Manual Backup

```bash
# Full database dump
docker compose exec postgres pg_dump -U postgres -Fc erc8004_backend > backup_$(date +%Y%m%d_%H%M%S).dump

# Schema only
docker compose exec postgres pg_dump -U postgres --schema-only erc8004_backend > schema_backup.sql

# Verify backup size
ls -lh backup_*.dump
```

#### Restore from Backup

```bash
# Stop all services writing to database
docker compose stop api-gateway event-processor action-workers ponder-indexers

# Restore database
docker compose exec -T postgres pg_restore -U postgres -d erc8004_backend -c < backup_20250130_120000.dump

# Run migrations (if needed)
cd rust-backend && sqlx migrate run && cd ..

# Restart services
docker compose start ponder-indexers event-processor action-workers api-gateway
```

### Redis Persistence

Redis is configured with AOF (Append-Only File) persistence:

```bash
# Check persistence status
docker compose exec redis redis-cli INFO persistence

# Trigger manual save
docker compose exec redis redis-cli BGSAVE

# Verify last save time
docker compose exec redis redis-cli LASTSAVE
```

### Configuration Backups

```bash
# Backup environment variables (NEVER commit to git)
cp .env .env.backup_$(date +%Y%m%d)

# Backup docker-compose configuration
cp docker-compose.yml docker-compose.backup.yml
```

---

## Rollback Procedures

### Application Rollback

#### Option 1: Docker Image Rollback

```bash
# Check current version
docker compose images api-gateway

# Pull previous version
docker pull your-registry/api-gateway:v1.2.3

# Update docker-compose.yml to use previous version
# image: your-registry/api-gateway:v1.2.3

# Restart service
docker compose up -d api-gateway
```

#### Option 2: Git Rollback + Rebuild

```bash
# Find last stable commit
git log --oneline -10

# Revert to stable commit
git checkout <commit-hash>

# Rebuild and restart
docker compose build api-gateway
docker compose up -d api-gateway
```

### Database Rollback

#### Revert Last Migration

```bash
# Check current version
cd rust-backend
sqlx migrate info

# Revert last migration
sqlx migrate revert

# Restart services
docker compose restart api-gateway event-processor
```

⚠️ **Warning**: Database rollbacks can cause data loss. Always backup first.

### Configuration Rollback

```bash
# Restore previous environment variables
cp .env.backup_20250130 .env

# Restart affected services
docker compose restart api-gateway event-processor action-workers
```

---

## Escalation

### Escalation Path

1. **On-call Engineer** (immediate response)
   - Contact: Slack `#on-call` channel
   - Response time: < 5 minutes

2. **Engineering Lead** (technical escalation)
   - Contact: Slack `#engineering-leads`
   - Response time: < 15 minutes

3. **Engineering Manager** (management escalation)
   - Contact: Email + Phone
   - Response time: < 30 minutes

4. **CTO** (executive escalation)
   - Contact: Phone only
   - Response time: < 1 hour

### When to Escalate

- **Immediate (P0)**: System completely down, data loss risk
- **Within 15 min (P1)**: Unable to resolve within SLA, partial outage
- **Within 1 hour (P2)**: Complex issue requiring specialist, unclear root cause
- **Next business day (P3)**: Non-urgent improvements, documentation issues

### Escalation Checklist

Before escalating, ensure you have:
- [ ] Incident severity classification
- [ ] Symptoms and impact description
- [ ] Diagnostic steps already taken
- [ ] Relevant logs and metrics
- [ ] Estimated time to resolution (if known)
- [ ] Workaround status (if any)

---

## Emergency Contacts

| Role | Contact | Availability |
|------|---------|--------------|
| On-call Engineer | Slack `@oncall` | 24/7 |
| Database Admin | Slack `@dba-team` | Business hours |
| Security Team | Email: security@8004.dev | 24/7 |
| RPC Provider Support | Alchemy/Infura support portals | 24/7 |

---

## Related Documentation

- **Troubleshooting**: [TROUBLESHOOTING.md](./TROUBLESHOOTING.md)
- **Circuit Breaker Guide**: [CIRCUIT_BREAKER_GUIDE.md](./CIRCUIT_BREAKER_GUIDE.md)
- **API Documentation**: [../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)
- **Database Schema**: [../../database/schema.md](../../database/schema.md)
- **Architecture Overview**: [../architecture/system-overview.md](../architecture/system-overview.md)

---

**Last Updated**: January 30, 2025
**Maintainer**: DevOps Team
**Version**: 1.0.0
