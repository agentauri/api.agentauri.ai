# Analytics Materialized Views - Performance Analysis

## Overview

This document provides performance analysis, optimization strategies, and best practices for the analytics materialized views system.

## View Performance Characteristics

### 1. action_metrics_hourly

**Purpose**: Hourly aggregated metrics by action type and status

**Size Estimation**:
- 3 action types × 3 statuses × 24 hours = ~216 rows/day
- 30 days retention = ~6,480 rows
- Row size: ~200 bytes
- **Total size**: ~1.3 MB (small)

**Query Performance**:
- Indexed queries: **< 1ms** (using idx_action_metrics_hourly_hour)
- Full scan: **< 5ms** (small dataset)
- Aggregation overhead: **Minimal** (pre-aggregated data)

**Refresh Performance**:
- Expected duration: **500ms - 2s** (depends on action_results volume)
- CPU usage: **Low** (simple aggregation)
- Locks: **None** (REFRESH MATERIALIZED VIEW CONCURRENTLY)

**Optimization Strategies**:
1. **Partial index on recent data**: Already implemented for last 30 days
2. **Archive old data**: Implement retention policy to delete rows older than 90 days
3. **Partition by month**: If dataset grows beyond 100K rows

### 2. trigger_performance_summary

**Purpose**: Per-trigger aggregate statistics

**Size Estimation**:
- 100 triggers (typical deployment) = 100 rows
- 1000 triggers (large deployment) = 1,000 rows
- Row size: ~300 bytes
- **Total size**: 30 KB - 300 KB (very small)

**Query Performance**:
- Primary key lookup: **< 1ms**
- Sorted queries (slowest triggers): **< 5ms**
- Joins with triggers table: **< 10ms**

**Refresh Performance**:
- Expected duration: **100ms - 500ms**
- CPU usage: **Low**
- Locks: **None** (REFRESH MATERIALIZED VIEW CONCURRENTLY)

**Optimization Strategies**:
1. **Index on frequently filtered columns**: Already implemented (success_rate, avg_duration_ms)
2. **Incremental refresh**: For very large trigger counts (1M+), consider incremental refresh strategy
3. **Denormalize trigger metadata**: Include trigger name in view to avoid joins

### 3. recent_failures

**Purpose**: Failed actions in last 24 hours

**Size Estimation**:
- 1% failure rate × 10,000 actions/day = 100 failures/day
- 24-hour window = ~100 rows (sliding)
- Row size: ~400 bytes (includes error_message, response_data)
- **Total size**: ~40 KB (very small)

**Query Performance**:
- Time-based queries: **< 1ms**
- Group by trigger_id: **< 5ms**
- Text search on error_message: **< 10ms** (small dataset)

**Refresh Performance**:
- Expected duration: **50ms - 200ms**
- CPU usage: **Very low**
- Locks: **None**

**Optimization Strategies**:
1. **No optimization needed**: Dataset is naturally small (24-hour window)
2. **Consider GIN index on error_message**: Only if text search is frequent
3. **Add response_data JSONB index**: If querying nested error details

### 4. action_type_distribution

**Purpose**: Action type usage over last 30 days

**Size Estimation**:
- 3 action types = 3 rows
- Row size: ~200 bytes
- **Total size**: ~600 bytes (tiny)

**Query Performance**:
- Full scan: **< 1ms**
- No optimization needed

**Refresh Performance**:
- Expected duration: **100ms - 500ms** (aggregates 30 days of data)
- CPU usage: **Low**

**Optimization Strategies**:
1. **No optimization needed**: Dataset is always 3 rows

## Refresh Strategy

### Recommended Schedule

**Production**: Every **5 minutes**
- Balances freshness with system load
- Aligns with typical dashboard refresh rates
- Minimal overhead (~3s total refresh time)

**Development**: Every **15 minutes**
- Reduces database load
- Acceptable for testing environments

**Low-Traffic**: Every **1 hour**
- Suitable for low-volume deployments
- Reduces unnecessary refreshes

### Implementation Options

#### Option 1: PostgreSQL pg_cron (Recommended)

```sql
-- Install pg_cron extension
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- Schedule refresh every 5 minutes
SELECT cron.schedule(
    'refresh-analytics',
    '*/5 * * * *',
    $$SELECT * FROM refresh_action_analytics()$$
);

-- Verify schedule
SELECT * FROM cron.job;

-- View job run history
SELECT * FROM cron.job_run_details ORDER BY start_time DESC LIMIT 10;
```

**Pros**:
- Native PostgreSQL solution
- No external dependencies
- Automatic retry on failure
- Detailed execution logs

**Cons**:
- Requires pg_cron extension (may not be available on all hosting providers)

#### Option 2: Application-Level Scheduler (Rust)

```rust
use tokio::time::{interval, Duration};
use sqlx::PgPool;

async fn schedule_analytics_refresh(pool: PgPool) {
    let mut interval = interval(Duration::from_secs(300)); // 5 minutes

    loop {
        interval.tick().await;

        match refresh_analytics(&pool).await {
            Ok(results) => {
                for result in results {
                    tracing::info!(
                        "Refreshed {}: {} rows in {}ms",
                        result.view_name,
                        result.rows_refreshed,
                        result.refresh_duration_ms
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to refresh analytics: {}", e);
            }
        }
    }
}

async fn refresh_analytics(pool: &PgPool) -> Result<Vec<RefreshResult>> {
    sqlx::query_as!(
        RefreshResult,
        "SELECT * FROM refresh_action_analytics()"
    )
    .fetch_all(pool)
    .await
}
```

**Pros**:
- Works on any PostgreSQL instance
- Integrated with application monitoring
- Can add custom retry logic and alerting

**Cons**:
- Requires application to be running
- Additional process management

#### Option 3: External Cron Job

```bash
#!/bin/bash
# /etc/cron.d/refresh-analytics
# Runs every 5 minutes

*/5 * * * * postgres psql -d agentauri_backend -c "SELECT * FROM refresh_action_analytics();" >> /var/log/analytics-refresh.log 2>&1
```

**Pros**:
- Simple
- Standard cron interface

**Cons**:
- Connection overhead on each run
- Less monitoring visibility
- Manual log management

### Monitoring Refresh Health

```sql
-- Check view staleness
SELECT
    view_name,
    row_count,
    age_minutes,
    CASE
        WHEN age_minutes > 10 THEN 'STALE (>10 min)'
        WHEN age_minutes > 5 THEN 'WARNING (>5 min)'
        ELSE 'FRESH'
    END AS staleness_status
FROM get_analytics_refresh_status()
ORDER BY age_minutes DESC;

-- Alert if views are stale (>10 minutes)
DO $$
DECLARE
    v_stale_views TEXT[];
BEGIN
    SELECT array_agg(view_name)
    INTO v_stale_views
    FROM get_analytics_refresh_status()
    WHERE age_minutes > 10;

    IF array_length(v_stale_views, 1) > 0 THEN
        RAISE WARNING 'Stale analytics views detected: %', v_stale_views;
    END IF;
END $$;
```

## Index Usage Analysis

### Verifying Index Effectiveness

```sql
-- Check index usage statistics
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan AS index_scans,
    idx_tup_read AS tuples_read,
    idx_tup_fetch AS tuples_fetched,
    pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
  AND tablename IN ('action_metrics_hourly', 'trigger_performance_summary', 'recent_failures', 'action_type_distribution')
ORDER BY tablename, indexname;

-- Identify unused indexes (idx_scan = 0)
SELECT
    schemaname || '.' || tablename AS table,
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
  AND tablename IN ('action_metrics_hourly', 'trigger_performance_summary', 'recent_failures', 'action_type_distribution')
  AND idx_scan = 0
ORDER BY pg_relation_size(indexrelid) DESC;
```

### Expected Index Usage

| View | Index | Expected Usage | Priority |
|------|-------|----------------|----------|
| action_metrics_hourly | idx_action_metrics_hourly_unique | Every refresh | Critical |
| action_metrics_hourly | idx_action_metrics_hourly_hour | Every time-range query | High |
| action_metrics_hourly | idx_action_metrics_hourly_recent | Dashboard queries | High |
| trigger_performance_summary | idx_trigger_performance_summary_unique | Every refresh | Critical |
| trigger_performance_summary | idx_trigger_performance_summary_success_rate | Health monitoring | Medium |
| recent_failures | idx_recent_failures_unique | Every refresh | Critical |
| recent_failures | idx_recent_failures_trigger_id | Failure analysis | High |

## Query Optimization Patterns

### Pattern 1: Time-Range Filtering

**Good** (uses index):
```sql
SELECT * FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours'
ORDER BY hour DESC;
```

**Bad** (full scan):
```sql
SELECT * FROM action_metrics_hourly
WHERE EXTRACT(HOUR FROM hour) = 12; -- Function on indexed column
```

### Pattern 2: Pagination

**Good** (efficient with LIMIT/OFFSET):
```sql
SELECT * FROM trigger_performance_summary
ORDER BY total_executions DESC
LIMIT 20 OFFSET 0;
```

**Bad** (fetches all rows):
```sql
SELECT * FROM trigger_performance_summary
ORDER BY total_executions DESC; -- No LIMIT
```

### Pattern 3: Aggregation

**Good** (pre-aggregated data):
```sql
SELECT action_type, SUM(execution_count)
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '7 days'
GROUP BY action_type;
```

**Bad** (re-aggregating aggregated data):
```sql
SELECT action_type, AVG(avg_duration_ms) -- AVG of AVG is incorrect
FROM action_metrics_hourly
GROUP BY action_type;
```

## Scaling Strategies

### Current Scale (< 1M action_results/month)

- **No optimization needed**
- Materialized views handle this easily
- Refresh time: < 3 seconds

### Medium Scale (1M - 10M action_results/month)

**Optimizations**:
1. **Incremental refresh**: Refresh only last 24-48 hours of data
2. **Parallel refresh**: Refresh views concurrently (already implemented)
3. **Partition action_results**: Partition by executed_at (monthly)

**Implementation**:
```sql
-- Partition action_results by month
CREATE TABLE action_results_2025_01 PARTITION OF action_results
FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

CREATE TABLE action_results_2025_02 PARTITION OF action_results
FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
```

### Large Scale (> 10M action_results/month)

**Optimizations**:
1. **Continuous aggregation**: Use TimescaleDB continuous aggregates
2. **Separate OLAP database**: Replicate to ClickHouse/BigQuery
3. **Sampling**: Aggregate 10% sample for high-cardinality views

**TimescaleDB Continuous Aggregate Example**:
```sql
CREATE MATERIALIZED VIEW action_metrics_hourly_continuous
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', executed_at) AS hour,
    action_type,
    status,
    COUNT(*) AS execution_count,
    AVG(duration_ms) AS avg_duration_ms
FROM action_results
GROUP BY time_bucket('1 hour', executed_at), action_type, status;

-- Automatic incremental refresh
SELECT add_continuous_aggregate_policy('action_metrics_hourly_continuous',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '5 minutes');
```

## Cost Analysis

### Storage Costs

| View | Rows (30 days) | Size | Annual Storage |
|------|----------------|------|----------------|
| action_metrics_hourly | ~6,500 | 1.3 MB | 16 MB |
| trigger_performance_summary | ~1,000 | 300 KB | 3.6 MB |
| recent_failures | ~100 | 40 KB | 480 KB |
| action_type_distribution | 3 | 600 B | 7.2 KB |
| **Total** | - | **~1.6 MB** | **~20 MB** |

**Conclusion**: Storage costs are negligible.

### Compute Costs

**Refresh overhead**:
- 288 refreshes/day (every 5 minutes)
- ~3 seconds per refresh
- **Total daily compute**: ~14 minutes
- **CPU usage**: < 0.1% of a single core

**Query overhead**:
- Dashboard queries: ~10 queries/minute
- Average query time: ~5ms
- **Total daily query time**: ~7 minutes

**Conclusion**: Compute costs are minimal.

## Troubleshooting

### Issue 1: Slow Refresh

**Symptoms**: refresh_action_analytics() takes > 10 seconds

**Diagnosis**:
```sql
-- Check action_results table size
SELECT pg_size_pretty(pg_total_relation_size('action_results'));

-- Check for missing indexes on action_results
SELECT * FROM pg_indexes WHERE tablename = 'action_results';

-- Analyze query plans
EXPLAIN ANALYZE REFRESH MATERIALIZED VIEW action_metrics_hourly;
```

**Solutions**:
1. Add indexes on action_results (executed_at, status, action_type)
2. Run VACUUM ANALYZE on action_results
3. Increase work_mem for refresh queries
4. Partition action_results table

### Issue 2: Stale Views

**Symptoms**: get_analytics_refresh_status() shows age_minutes > 10

**Diagnosis**:
```sql
-- Check if pg_cron job is running
SELECT * FROM cron.job WHERE jobname = 'refresh-analytics';

-- Check for errors
SELECT * FROM cron.job_run_details
WHERE jobid = (SELECT jobid FROM cron.job WHERE jobname = 'refresh-analytics')
ORDER BY start_time DESC LIMIT 5;
```

**Solutions**:
1. Verify pg_cron is enabled
2. Check database connection limits
3. Review application scheduler logs
4. Manually run refresh_action_analytics()

### Issue 3: Missing Unique Index Error

**Symptoms**: "cannot refresh materialized view ... concurrently"

**Diagnosis**:
```sql
-- Check for unique indexes
SELECT indexname FROM pg_indexes
WHERE tablename = 'action_metrics_hourly'
  AND indexdef LIKE '%UNIQUE%';
```

**Solution**:
```sql
-- Re-create unique indexes if missing
CREATE UNIQUE INDEX idx_action_metrics_hourly_unique
    ON action_metrics_hourly(hour, action_type, status);
```

## Best Practices

1. **Monitor refresh performance**: Set up alerts if refresh time > 5 seconds
2. **Validate data consistency**: Periodically compare view data with source tables
3. **Archive old data**: Implement retention policies on action_results (90+ days)
4. **Use prepared statements**: Cache query plans for frequent dashboard queries
5. **Test before deploying**: Run refresh on production-sized data in staging
6. **Document query patterns**: Share analytics.sql with dashboard developers
7. **Version control views**: Track view schema changes in migrations
8. **Monitor index usage**: Drop unused indexes to reduce storage and refresh time

## Maintenance Checklist

### Daily
- [ ] Check view staleness (age_minutes < 10)
- [ ] Review error logs for failed refreshes

### Weekly
- [ ] Analyze query performance (slow query log)
- [ ] Review index usage statistics
- [ ] Validate data accuracy (spot checks)

### Monthly
- [ ] Archive old action_results data (> 90 days)
- [ ] Review storage growth trends
- [ ] Update refresh schedule if needed
- [ ] Review and optimize slow queries

### Quarterly
- [ ] Performance benchmark tests
- [ ] Evaluate scaling strategies
- [ ] Review retention policies
- [ ] Update this document with new learnings
