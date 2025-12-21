# Audit Logging

> **Status**: Implemented
> **Last Updated**: December 2024

This document describes the audit logging system for A2A Protocol task operations, including event types, data captured, and query patterns.

## Overview

The A2A audit logging system provides a complete audit trail for all task operations. It records:

- Task lifecycle events (created, started, completed, failed, cancelled, timeout)
- Actor information (who initiated the action)
- Cost and performance metrics
- Error details for failed operations

## Database Schema

### a2a_task_audit_log Table

```sql
CREATE TABLE a2a_task_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES a2a_tasks(id) ON DELETE CASCADE,
    organization_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id TEXT,
    tool TEXT,
    cost_micro_usdc BIGINT,
    duration_ms BIGINT,
    error_message TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_a2a_audit_task ON a2a_task_audit_log(task_id);
CREATE INDEX idx_a2a_audit_org ON a2a_task_audit_log(organization_id);
CREATE INDEX idx_a2a_audit_created ON a2a_task_audit_log(created_at DESC);
CREATE INDEX idx_a2a_audit_event ON a2a_task_audit_log(event_type);
```

## Event Types

| Event | Description | Recorded Data |
|-------|-------------|---------------|
| `created` | Task was created via tasks/send | actor, tool |
| `started` | Task processor began execution | tool |
| `completed` | Task finished successfully | tool, cost, duration |
| `failed` | Task execution failed | tool, duration, error_message |
| `cancelled` | Task was cancelled by user | actor |
| `timeout` | Task execution timed out | tool, duration |

### Event Flow

```
created → started → completed
              ↓
           failed
              ↓
           timeout

cancelled (from created or started)
```

## Actor Types

| Actor Type | Description | actor_id Value |
|------------|-------------|----------------|
| `user` | Authenticated user via JWT | User UUID |
| `api_key` | Request via API key | API key prefix (e.g., `sk_live_abc`) |
| `system` | Background task processor | `processor` |

## Implementation

The audit service is implemented in `rust-backend/crates/api-gateway/src/services/a2a_audit.rs`.

### Logging Functions

```rust
// Log task creation (called from handlers)
A2aAuditService::log_created(
    pool,
    &task_id,
    org_id,
    &tool,
    AuditActor::ApiKey(key_prefix),
).await?;

// Log task started (called from task processor)
A2aAuditService::log_started(
    pool,
    &task_id,
    org_id,
    &tool,
).await?;

// Log task completed (called from task processor)
A2aAuditService::log_completed(
    pool,
    &task_id,
    org_id,
    &tool,
    cost_micro_usdc,
    duration_ms,
).await?;

// Log task failed (called from task processor)
A2aAuditService::log_failed(
    pool,
    &task_id,
    org_id,
    &tool,
    duration_ms,
    &error_message,
).await?;

// Log task timeout (called from task processor)
A2aAuditService::log_timeout(
    pool,
    &task_id,
    org_id,
    &tool,
    duration_ms,
).await?;

// Log task cancelled (called from handlers)
A2aAuditService::log_cancelled(
    pool,
    &task_id,
    org_id,
    AuditActor::User(user_id),
).await?;
```

## Query Examples

### Get All Events for a Task

```sql
SELECT *
FROM a2a_task_audit_log
WHERE task_id = '550e8400-e29b-41d4-a716-446655440000'
ORDER BY created_at ASC;
```

### Get Failed Tasks for an Organization

```sql
SELECT
    task_id,
    tool,
    error_message,
    duration_ms,
    created_at
FROM a2a_task_audit_log
WHERE organization_id = 'org-123'
  AND event_type = 'failed'
ORDER BY created_at DESC
LIMIT 100;
```

### Get Task Performance Summary

```sql
SELECT
    tool,
    COUNT(*) FILTER (WHERE event_type = 'completed') as completed_count,
    COUNT(*) FILTER (WHERE event_type = 'failed') as failed_count,
    COUNT(*) FILTER (WHERE event_type = 'timeout') as timeout_count,
    AVG(duration_ms) FILTER (WHERE event_type = 'completed') as avg_duration_ms,
    SUM(cost_micro_usdc) FILTER (WHERE event_type = 'completed') as total_cost_micro_usdc
FROM a2a_task_audit_log
WHERE organization_id = 'org-123'
  AND created_at >= NOW() - INTERVAL '7 days'
GROUP BY tool
ORDER BY completed_count DESC;
```

### Get Top Error Messages

```sql
SELECT
    error_message,
    COUNT(*) as count,
    array_agg(DISTINCT tool) as affected_tools
FROM a2a_task_audit_log
WHERE event_type = 'failed'
  AND created_at >= NOW() - INTERVAL '24 hours'
GROUP BY error_message
ORDER BY count DESC
LIMIT 10;
```

### Get Hourly Task Volume

```sql
SELECT
    date_trunc('hour', created_at) as hour,
    event_type,
    COUNT(*) as count
FROM a2a_task_audit_log
WHERE organization_id = 'org-123'
  AND created_at >= NOW() - INTERVAL '24 hours'
GROUP BY hour, event_type
ORDER BY hour DESC;
```

## Compliance Considerations

### Data Retention

Audit logs should be retained according to your organization's compliance requirements:
- **Default**: 90 days
- **SOC 2**: 1 year minimum
- **GDPR**: Consider anonymization after retention period

### Data Cleanup Query

```sql
-- Delete audit logs older than 90 days
DELETE FROM a2a_task_audit_log
WHERE created_at < NOW() - INTERVAL '90 days';
```

### Anonymization Query

```sql
-- Anonymize actor_id for old logs
UPDATE a2a_task_audit_log
SET actor_id = 'anonymized'
WHERE created_at < NOW() - INTERVAL '1 year'
  AND actor_id IS NOT NULL;
```

## Integration with Monitoring

Audit events can be exported to external monitoring systems:

### Prometheus Metrics (Derived)

```promql
# Calculate success rate from audit logs
# (This is computed by the application, not from DB)
a2a_tasks_completed_total / (a2a_tasks_completed_total + a2a_tasks_failed_total)
```

### Alerting Rules

Consider setting alerts for:
- High failure rate (> 10% in 15 minutes)
- Timeout spikes
- Unusual error patterns

## Security Considerations

1. **Access Control**: Audit logs should be read-only for application users
2. **Sensitive Data**: Never log sensitive data (API keys, credentials)
3. **Actor Tracking**: Always identify who performed actions
4. **Immutability**: Audit logs should not be editable after creation

## Related Documentation

- [A2A Protocol Integration](../protocols/A2A_INTEGRATION.md)
- [Metrics](./METRICS.md)
- [Troubleshooting](./TROUBLESHOOTING.md)

---

**Last Updated**: December 21, 2024
