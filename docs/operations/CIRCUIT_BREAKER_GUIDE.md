# Circuit Breaker Practical Guide

**For**: Operations, DevOps, and Backend Engineers
**Date**: November 30, 2025
**Status**: Production-Ready ✅

## Quick Reference

### Check if Triggers Have Open Circuits

```sql
-- Show all triggers with open or half-open circuits
SELECT
    id,
    name,
    organization_id,
    circuit_breaker_state->>'state' AS state,
    (circuit_breaker_state->>'failure_count')::int AS failures,
    circuit_breaker_state->>'opened_at' AS opened_at,
    NOW() - (circuit_breaker_state->>'opened_at')::timestamptz AS time_open
FROM triggers
WHERE circuit_breaker_state->>'state' IN ('Open', 'HalfOpen')
ORDER BY opened_at DESC;
```

### Manually Reset a Circuit Breaker

```sql
-- Reset circuit to Closed state (use with caution)
UPDATE triggers
SET circuit_breaker_state = '{
    "state": "Closed",
    "failure_count": 0,
    "half_open_calls": 0
}'::jsonb
WHERE id = 'trigger-id-here';
```

### Check Recent Failures

```sql
-- Show triggers with recent failures (even if not open)
SELECT
    id,
    name,
    (circuit_breaker_state->>'failure_count')::int AS consecutive_failures,
    circuit_breaker_state->>'last_failure_time' AS last_failure,
    circuit_breaker_state->>'state' AS current_state,
    (circuit_breaker_config->>'failure_threshold')::int AS threshold
FROM triggers
WHERE (circuit_breaker_state->>'failure_count')::int > 0
ORDER BY (circuit_breaker_state->>'failure_count')::int DESC
LIMIT 20;
```

---

## Common Scenarios

### Scenario 1: Production Alert - 5 Triggers Have Open Circuits

**Alert Message**:
```
⚠️ 5 triggers have open circuits (disabled due to repeated failures)
```

**Investigation Steps**:

1. **Identify affected triggers**:
```sql
SELECT
    id,
    name,
    organization_id,
    circuit_breaker_state->>'opened_at' AS opened_at,
    NOW() - (circuit_breaker_state->>'opened_at')::timestamptz AS time_open
FROM triggers
WHERE circuit_breaker_state->>'state' = 'Open'
ORDER BY opened_at;
```

2. **Check error logs**:
```bash
# Search for error logs related to failing triggers
grep "Trigger evaluation failed" /var/log/event-processor/app.log | tail -100
```

3. **Analyze failure patterns**:
```sql
-- Check action_results for common failure reasons
SELECT
    action_type,
    error_message,
    COUNT(*) as error_count
FROM action_results
WHERE status = 'failed'
    AND created_at > NOW() - INTERVAL '1 hour'
    AND trigger_id IN (
        SELECT id FROM triggers
        WHERE circuit_breaker_state->>'state' = 'Open'
    )
GROUP BY action_type, error_message
ORDER BY error_count DESC
LIMIT 10;
```

4. **Root cause examples and fixes**:

| Error Message | Root Cause | Fix |
|---------------|------------|-----|
| `Telegram API rate limit exceeded` | Too many notifications | Reduce notification frequency or batch updates |
| `Invalid webhook URL: connection refused` | External service down | Verify service health, update webhook URL |
| `Database connection timeout` | Database overload | Scale database or optimize queries |
| `Invalid trigger configuration` | Bad user input | Update trigger configuration |

5. **Manual recovery** (after fixing root cause):
```sql
-- Reset circuits for specific triggers after fixing the issue
UPDATE triggers
SET circuit_breaker_state = '{
    "state": "Closed",
    "failure_count": 0,
    "half_open_calls": 0
}'::jsonb
WHERE id IN ('trigger-1', 'trigger-2', 'trigger-3');
```

---

### Scenario 2: Circuit Stuck in Open State for Hours

**Problem**: Circuit breaker opened 3 hours ago, should have transitioned to Half-Open after 1 hour.

**Investigation**:

1. **Check recovery timeout configuration**:
```sql
SELECT
    id,
    name,
    circuit_breaker_config->>'recovery_timeout_seconds' AS timeout_seconds,
    circuit_breaker_state->>'opened_at' AS opened_at,
    NOW() - (circuit_breaker_state->>'opened_at')::timestamptz AS time_open
FROM triggers
WHERE id = 'stuck-trigger-id';
```

**Expected output**:
```
id              | timeout_seconds | opened_at              | time_open
----------------|-----------------|------------------------|----------
stuck-trigger   | 3600            | 2025-11-30 14:00:00    | 03:15:23
```

2. **Check if events are still being processed**:
```sql
-- Check if new events are arriving for this trigger's chain/registry
SELECT COUNT(*) AS recent_events
FROM events
WHERE chain_id = (SELECT chain_id FROM triggers WHERE id = 'stuck-trigger-id')
    AND registry = (SELECT registry FROM triggers WHERE id = 'stuck-trigger-id')
    AND created_at > NOW() - INTERVAL '1 hour';
```

**If recent_events = 0**: No events arriving, circuit won't transition (needs event to trigger transition)

**Fix**: Wait for next event, or manually reset circuit

**If recent_events > 0**: Events arriving but circuit not transitioning

**Possible causes**:
- Event processor crashed/restarted
- Circuit breaker bug
- Database state corruption

**Fix**: Manually reset circuit and investigate logs

3. **Force transition to Half-Open** (if needed):
```sql
UPDATE triggers
SET circuit_breaker_state = jsonb_set(
    circuit_breaker_state,
    '{state}',
    '"HalfOpen"'
)
WHERE id = 'stuck-trigger-id';
```

---

### Scenario 3: Flapping Circuit (Opens and Closes Repeatedly)

**Symptoms**: Circuit transitions between Closed and Open frequently (every few minutes)

**Investigation**:

1. **Check transition frequency**:
```bash
# Search logs for state transitions
grep "Circuit breaker transitioning" /var/log/event-processor/app.log | grep "trigger-id" | tail -20
```

**Example output showing flapping**:
```
2025-11-30 15:00:00 WARN Circuit breaker transitioning to Open trigger_id=xxx
2025-11-30 16:00:05 INFO Circuit breaker transitioning to Half-Open trigger_id=xxx
2025-11-30 16:00:06 WARN Circuit breaker transitioning to Open trigger_id=xxx
2025-11-30 17:00:05 INFO Circuit breaker transitioning to Half-Open trigger_id=xxx
2025-11-30 17:00:06 INFO Circuit breaker transitioning to Closed trigger_id=xxx
2025-11-30 17:05:00 WARN Circuit breaker transitioning to Open trigger_id=xxx
```

2. **Root cause**: Intermittent failures (e.g., network issues, rate limits)

3. **Solutions**:

**Option A: Increase failure threshold** (more tolerant)
```sql
-- Increase threshold from 10 to 20 failures
UPDATE triggers
SET circuit_breaker_config = jsonb_set(
    circuit_breaker_config,
    '{failure_threshold}',
    '20'
)
WHERE id = 'flapping-trigger-id';
```

**Option B: Increase recovery timeout** (longer cooldown)
```sql
-- Increase timeout from 1 hour to 4 hours
UPDATE triggers
SET circuit_breaker_config = jsonb_set(
    circuit_breaker_config,
    '{recovery_timeout_seconds}',
    '14400'
)
WHERE id = 'flapping-trigger-id';
```

**Option C: Fix underlying issue** (best approach)
- Add retry logic to webhook calls
- Implement exponential backoff
- Use fallback webhook URLs

---

### Scenario 4: Circuit Breaker Not Opening Despite Failures

**Problem**: Trigger has failed 50 times but circuit is still Closed

**Investigation**:

1. **Check if failures are consecutive**:
```sql
SELECT
    circuit_breaker_state->>'failure_count' AS count,
    circuit_breaker_state->>'state' AS state,
    (circuit_breaker_config->>'failure_threshold')::int AS threshold
FROM triggers
WHERE id = 'non-opening-trigger';
```

**Key insight**: Circuit only opens on **consecutive** failures. If there's a success in between, the counter resets.

**Example sequence**:
```
Fail, Fail, Fail, Success → Counter resets to 0 ❌
Fail, Fail, Fail, Fail, Fail → Counter = 5 ✅
```

2. **Check if failures are actually being recorded**:
```bash
# Search logs for "record_failure" calls
grep "record_failure" /var/log/event-processor/app.log | grep "trigger-id" | tail -20
```

3. **Verify failure threshold**:
```sql
-- Check current threshold
SELECT
    circuit_breaker_config->>'failure_threshold' AS threshold
FROM triggers
WHERE id = 'non-opening-trigger';
```

**If threshold is too high** (e.g., 100), lower it:
```sql
UPDATE triggers
SET circuit_breaker_config = jsonb_set(
    circuit_breaker_config,
    '{failure_threshold}',
    '10'
)
WHERE id = 'non-opening-trigger';
```

---

### Scenario 5: All Triggers in Organization Have Open Circuits

**Problem**: 100% of triggers for org_123 have open circuits

**Investigation**:

1. **Identify scope of issue**:
```sql
SELECT
    organization_id,
    COUNT(*) FILTER (WHERE circuit_breaker_state->>'state' = 'Open') AS open_circuits,
    COUNT(*) AS total_triggers,
    ROUND(100.0 * COUNT(*) FILTER (WHERE circuit_breaker_state->>'state' = 'Open') / COUNT(*), 2) AS percentage_open
FROM triggers
GROUP BY organization_id
HAVING COUNT(*) FILTER (WHERE circuit_breaker_state->>'state' = 'Open') > 0
ORDER BY percentage_open DESC;
```

2. **Check common error messages**:
```sql
SELECT
    error_message,
    COUNT(*) as occurrences
FROM action_results
WHERE status = 'failed'
    AND trigger_id IN (
        SELECT id FROM triggers WHERE organization_id = 'org_123'
    )
    AND created_at > NOW() - INTERVAL '2 hours'
GROUP BY error_message
ORDER BY occurrences DESC;
```

3. **Likely causes and fixes**:

| Cause | Fix |
|-------|-----|
| **Organization-wide service outage** | Wait for service recovery, then reset circuits |
| **Invalid organization API keys** | Rotate API keys, update configurations |
| **Rate limit exceeded at org level** | Upgrade plan or reduce trigger frequency |
| **Database migration broke schema** | Rollback or fix migration |

4. **Bulk reset** (after fixing root cause):
```sql
UPDATE triggers
SET circuit_breaker_state = '{
    "state": "Closed",
    "failure_count": 0,
    "half_open_calls": 0
}'::jsonb
WHERE organization_id = 'org_123'
    AND circuit_breaker_state->>'state' = 'Open';
```

---

## Configuration Tuning Guide

### Default Configuration

```json
{
    "failure_threshold": 10,
    "recovery_timeout_seconds": 3600,
    "half_open_max_calls": 1
}
```

### Recommended Configurations by Use Case

#### High-Frequency Triggers (100+ events/hour)

**Problem**: 10 failures might happen in seconds
**Solution**: Higher threshold, shorter timeout

```sql
UPDATE triggers
SET circuit_breaker_config = '{
    "failure_threshold": 50,
    "recovery_timeout_seconds": 600,
    "half_open_max_calls": 1
}'::jsonb
WHERE name LIKE 'High Frequency%';
```

#### Critical Triggers (must stay enabled)

**Problem**: Can't afford circuit to open
**Solution**: Very high threshold, long timeout

```sql
UPDATE triggers
SET circuit_breaker_config = '{
    "failure_threshold": 100,
    "recovery_timeout_seconds": 7200,
    "half_open_max_calls": 5
}'::jsonb
WHERE name LIKE 'Critical%';
```

#### Test/Development Triggers

**Problem**: Failures expected during development
**Solution**: Lower threshold for faster feedback

```sql
UPDATE triggers
SET circuit_breaker_config = '{
    "failure_threshold": 3,
    "recovery_timeout_seconds": 60,
    "half_open_max_calls": 1
}'::jsonb
WHERE organization_id IN (
    SELECT id FROM organizations WHERE slug LIKE 'test-%'
);
```

#### External API Triggers (flaky services)

**Problem**: External APIs have intermittent issues
**Solution**: More tolerant threshold, multiple test calls

```sql
UPDATE triggers
SET circuit_breaker_config = '{
    "failure_threshold": 20,
    "recovery_timeout_seconds": 1800,
    "half_open_max_calls": 3
}'::jsonb
WHERE name LIKE '%webhook%';
```

---

## Monitoring Best Practices

### Metrics to Track (Future Prometheus Integration)

1. **Circuit Breaker State Distribution**:
```promql
count by (state) (circuit_breaker_state)
```

**Target**: <5% of triggers in Open state

2. **State Transition Rate**:
```promql
rate(circuit_breaker_state_transitions_total[5m])
```

**Alert if**: >10 transitions/minute (indicates flapping)

3. **Open Circuit Duration**:
```promql
histogram_quantile(0.95, circuit_breaker_open_duration_seconds)
```

**Alert if**: p95 >2 hours (circuits stuck open)

4. **Failure Rate by Trigger**:
```promql
rate(circuit_breaker_failures_total[5m]) by (trigger_id)
```

**Alert if**: Any trigger >1 failure/second

### Log Queries

**Top 10 Failing Triggers**:
```bash
grep "Trigger evaluation failed" /var/log/event-processor/app.log \
    | grep -oP 'trigger_id=\K[^ ]+' \
    | sort | uniq -c | sort -rn | head -10
```

**Circuit State Changes in Last Hour**:
```bash
grep "Circuit breaker transitioning" /var/log/event-processor/app.log \
    | grep -P "$(date -u --date='1 hour ago' +%Y-%m-%d)" \
    | tail -20
```

---

## Troubleshooting Checklist

### When Circuit Breaker Opens

- [ ] Check error logs for failure reason
- [ ] Verify external services are healthy
- [ ] Check database connection pool status
- [ ] Review recent code deployments
- [ ] Check action_results table for error patterns
- [ ] Verify API keys/credentials are valid
- [ ] Check rate limits on external APIs
- [ ] Review trigger configuration for errors

### When Circuit Breaker Doesn't Open

- [ ] Verify failure_threshold is reasonable
- [ ] Check if failures are consecutive
- [ ] Review logs for "record_failure" calls
- [ ] Verify circuit_breaker_state is being updated
- [ ] Check database triggers are firing
- [ ] Review event processor health

### When Circuit Breaker Stuck in Open

- [ ] Check recovery_timeout_seconds configuration
- [ ] Verify events are still arriving for this trigger
- [ ] Check event processor is running
- [ ] Review database connection health
- [ ] Check for circuit_breaker_state corruption

### When Circuit Breaker Flaps

- [ ] Analyze failure patterns (intermittent vs persistent)
- [ ] Increase failure_threshold if too sensitive
- [ ] Increase recovery_timeout_seconds for longer cooldown
- [ ] Fix underlying intermittent issue (preferred)
- [ ] Consider implementing retry logic

---

## Emergency Procedures

### Emergency: Disable All Circuit Breakers

**Use case**: Circuit breakers are causing more harm than good (e.g., widespread outage)

```sql
-- Set all thresholds to impossibly high value (effectively disabled)
UPDATE triggers
SET circuit_breaker_config = jsonb_set(
    circuit_breaker_config,
    '{failure_threshold}',
    '999999'
)
WHERE enabled = true;

-- Reset all open circuits
UPDATE triggers
SET circuit_breaker_state = '{
    "state": "Closed",
    "failure_count": 0,
    "half_open_calls": 0
}'::jsonb
WHERE circuit_breaker_state->>'state' IN ('Open', 'HalfOpen');
```

**⚠️ WARNING**: This disables all failure protection. Only use in emergencies.

### Emergency: Reset All Circuits for Organization

**Use case**: Organization-wide service recovered, need to quickly re-enable triggers

```sql
UPDATE triggers
SET circuit_breaker_state = '{
    "state": "Closed",
    "failure_count": 0,
    "half_open_calls": 0
}'::jsonb
WHERE organization_id = 'affected-org-id'
    AND circuit_breaker_state->>'state' IN ('Open', 'HalfOpen');
```

---

## Performance Impact

### Database Overhead

**Per trigger per event**:
- Circuit breaker creation: ~2ms (reads config + state)
- Call allowed check: <0.1ms (in-memory)
- State update: ~2ms (writes state on transitions only)

**Total**: ~2-4ms per trigger per event

### Optimization Opportunities

1. **Cache circuit breakers in memory** (Week 22+):
   - Reduce creation overhead from 2ms to <0.1ms
   - 95% reduction in database queries

2. **Batch state updates**:
   - Update multiple triggers in single transaction
   - Reduce write contention

3. **Async state persistence**:
   - Don't block evaluation on database writes
   - Use background task for persistence

---

## Best Practices Summary

### Configuration

✅ **DO**:
- Set failure_threshold based on trigger frequency
- Use longer recovery_timeout for critical triggers
- Monitor circuit state transitions
- Document custom configurations

❌ **DON'T**:
- Set threshold too low (causes false positives)
- Set timeout too short (causes flapping)
- Manually reset circuits without fixing root cause
- Ignore open circuits for extended periods

### Operations

✅ **DO**:
- Investigate root cause before resetting circuits
- Monitor open circuit percentage
- Set up alerts for stuck circuits
- Review failure patterns weekly

❌ **DON'T**:
- Bulk reset all circuits without investigation
- Disable circuit breakers in production
- Ignore flapping circuits
- Leave circuits open for >24 hours

### Debugging

✅ **DO**:
- Check logs first
- Analyze failure patterns in action_results
- Verify external service health
- Review recent code changes

❌ **DON'T**:
- Assume circuit breaker is broken
- Reset circuits blindly
- Ignore warning logs
- Skip root cause analysis

---

## Support Escalation

### Level 1: Operations Team

**Handles**:
- Checking circuit breaker states
- Resetting circuits after confirmed fixes
- Basic log analysis
- Routine monitoring alerts

**Escalates to Level 2 if**:
- Root cause unclear
- Circuits flapping repeatedly
- Widespread failures (>20% of triggers)
- Database state corruption suspected

### Level 2: Backend Engineering

**Handles**:
- Deep log analysis
- Database state investigation
- Configuration tuning
- Code-level debugging

**Escalates to Level 3 if**:
- Circuit breaker code bug suspected
- Database migration issue
- Infrastructure-level problem

### Level 3: Senior Engineering

**Handles**:
- Circuit breaker code fixes
- Database schema changes
- Infrastructure scaling
- Architecture decisions

---

**Last Updated**: November 30, 2025
**Maintained By**: Backend Engineering Team
**Next Review**: December 30, 2025
