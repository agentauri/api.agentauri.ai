# Analytics System - Quick Reference Guide

## TL;DR

**4 Materialized Views** + **27 Ready-to-Use Queries** + **2 Helper Functions**

Refresh every 5 minutes. Query latency < 10ms. Storage: ~1.6 MB.

## Quick Commands

### Check View Status
```sql
SELECT * FROM get_analytics_refresh_status();
```

### Manual Refresh
```sql
SELECT * FROM refresh_action_analytics();
```

### Top 5 Queries You'll Use Daily

#### 1. System Health (Last 24 Hours)
```sql
SELECT * FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours'
ORDER BY hour DESC;
```

#### 2. Trigger Success Rates
```sql
SELECT trigger_id, success_rate, total_executions
FROM trigger_performance_summary
WHERE success_rate < 95
ORDER BY success_rate ASC;
```

#### 3. Recent Errors
```sql
SELECT trigger_id, error_message, COUNT(*)
FROM recent_failures
GROUP BY trigger_id, error_message
ORDER BY COUNT(*) DESC
LIMIT 10;
```

#### 4. Slowest Triggers
```sql
SELECT trigger_id, p95_duration_ms, total_executions
FROM trigger_performance_summary
ORDER BY p95_duration_ms DESC
LIMIT 10;
```

#### 5. Action Type Distribution
```sql
SELECT action_type, total_executions, percentage, success_rate
FROM action_type_distribution
ORDER BY total_executions DESC;
```

## 4 Materialized Views

| View | Purpose | Rows | Refresh Time |
|------|---------|------|--------------|
| `action_metrics_hourly` | Time-series metrics | ~6,500 | 500ms - 2s |
| `trigger_performance_summary` | Per-trigger stats | ~1,000 | 100ms - 500ms |
| `recent_failures` | Last 24h errors | ~100 | 50ms - 200ms |
| `action_type_distribution` | Usage by type | 3 | 100ms - 500ms |

## Common Patterns

### Filter by Time Range
```sql
WHERE hour > NOW() - INTERVAL '7 days'
```

### Filter by Organization
```sql
WHERE t.organization_id = 'org_abc123'
```

### Pagination
```sql
LIMIT 20 OFFSET 0
```

### Sort by Performance
```sql
ORDER BY p95_duration_ms DESC NULLS LAST
```

## API Endpoint Mapping

| Endpoint | View | Query Pattern |
|----------|------|---------------|
| `GET /analytics/actions/success-rate` | action_metrics_hourly | SUM aggregation by action_type |
| `GET /analytics/triggers/slowest` | trigger_performance_summary | ORDER BY p95_duration_ms DESC |
| `GET /analytics/failures/recent` | recent_failures | WHERE executed_at DESC |
| `GET /analytics/health/summary` | action_metrics_hourly | SUM over 24h window |

See `analytics_api_integration.md` for complete endpoint specifications.

## Refresh Schedule

**Production**: Every 5 minutes (pg_cron recommended)
```sql
SELECT cron.schedule('refresh-analytics', '*/5 * * * *',
    $$SELECT * FROM refresh_action_analytics()$$);
```

**Development**: Every 15 minutes
**Low-Traffic**: Every 1 hour

## Performance Tips

1. **Always use time ranges**: `WHERE hour > NOW() - INTERVAL '...'`
2. **Limit results**: Add `LIMIT` to all queries
3. **Use indexed columns**: hour, action_type, trigger_id, success_rate
4. **Avoid full scans**: Query materialized views, not action_results directly
5. **Cache client-side**: 5-minute cache aligns with refresh schedule

## Alerting Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| View staleness | > 10 min | > 30 min |
| Success rate | < 95% | < 90% |
| P95 latency | > 1s | > 5s |
| Refresh time | > 5s | > 10s |

## Troubleshooting

### Views are stale (age_minutes > 10)
```sql
-- Check refresh status
SELECT * FROM get_analytics_refresh_status();

-- Manual refresh
SELECT * FROM refresh_action_analytics();

-- Check for errors
SELECT * FROM cron.job_run_details WHERE jobname = 'refresh-analytics';
```

### Slow queries (> 100ms)
```sql
-- Analyze query plan
EXPLAIN ANALYZE <your_query>;

-- Check index usage
SELECT * FROM pg_stat_user_indexes
WHERE tablename = 'action_metrics_hourly';

-- Rebuild indexes if needed
REINDEX TABLE action_metrics_hourly;
```

### View not updating
```sql
-- Check if pg_cron is enabled
SELECT * FROM cron.job WHERE jobname = 'refresh-analytics';

-- Check action_results has new data
SELECT MAX(executed_at) FROM action_results;

-- Force refresh
REFRESH MATERIALIZED VIEW CONCURRENTLY action_metrics_hourly;
```

## File Locations

| File | Path |
|------|------|
| Migration | `/database/migrations/20251130000002_add_analytics_views.sql` |
| Queries | `/database/queries/analytics.sql` |
| Tests | `/database/tests/test-analytics.sql` |
| API Guide | `/database/queries/analytics_api_integration.md` |
| Performance | `/database/queries/analytics_performance_analysis.md` |

## Documentation

- **API Integration**: `analytics_api_integration.md` (15+ endpoint specs)
- **Performance Analysis**: `analytics_performance_analysis.md` (optimization guide)
- **Implementation Summary**: `ANALYTICS_IMPLEMENTATION_SUMMARY.md` (complete overview)
- **This Guide**: `ANALYTICS_QUICK_REFERENCE.md` (you are here)

## Testing

Run comprehensive test suite:
```bash
psql erc8004_backend < database/tests/test-analytics.sql
```

Expected: **26/26 tests passed**

## Support

- Slack: #analytics-backend
- Email: backend-team@8004.dev
- Docs: https://docs.8004.dev/analytics

---

**Last Updated**: 2025-11-30
**Status**: Production Ready
