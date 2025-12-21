# Metrics & Observability

> **Status**: Implemented
> **Last Updated**: December 2024

This document describes the metrics and observability features for the AgentAuri backend, with focus on A2A Protocol task processing metrics.

## Overview

The system uses the `metrics` crate for Prometheus-compatible metric emission. All metrics are exposed via the standard `/metrics` endpoint for scraping by Prometheus.

## A2A Task Metrics

### Counters

#### a2a.tasks.started

Number of tasks that started processing.

**Labels**:
- `tool`: Tool name (e.g., "getReputationSummary")

**Example**:
```promql
# Total tasks started
sum(a2a_tasks_started_total)

# Tasks started per tool (last hour)
increase(a2a_tasks_started_total[1h])
```

---

#### a2a.tasks.completed

Number of tasks that completed successfully.

**Labels**:
- `tool`: Tool name

**Example**:
```promql
# Success rate per tool
sum by (tool) (rate(a2a_tasks_completed_total[5m])) /
sum by (tool) (rate(a2a_tasks_started_total[5m]))
```

---

#### a2a.tasks.failed

Number of tasks that failed.

**Labels**:
- `tool`: Tool name
- `reason`: Failure reason (`error` or `timeout`)

**Example**:
```promql
# Failure rate (last 15 minutes)
sum(increase(a2a_tasks_failed_total[15m])) /
sum(increase(a2a_tasks_started_total[15m]))

# Timeout rate
sum by (tool) (rate(a2a_tasks_failed_total{reason="timeout"}[5m]))
```

### Histograms

#### a2a.tasks.duration_ms

Task execution duration in milliseconds.

**Labels**:
- `tool`: Tool name

**Example**:
```promql
# 95th percentile duration per tool
histogram_quantile(0.95,
  sum by (tool, le) (rate(a2a_tasks_duration_ms_bucket[5m]))
)

# Average duration per tool
sum by (tool) (rate(a2a_tasks_duration_ms_sum[5m])) /
sum by (tool) (rate(a2a_tasks_duration_ms_count[5m]))
```

### Gauges

#### a2a.processor.tasks_claimed

Number of tasks claimed in the current processing cycle.

**Example**:
```promql
# Current tasks being processed
a2a_processor_tasks_claimed
```

## Metric Implementation

Metrics are emitted from the A2A task processor in `a2a_task_processor.rs`:

```rust
use metrics::{counter, gauge, histogram};

// On task start
counter!("a2a.tasks.started", "tool" => task.tool.clone()).increment(1);

// On task complete
counter!("a2a.tasks.completed", "tool" => task.tool.clone()).increment(1);
histogram!("a2a.tasks.duration_ms", "tool" => task.tool.clone())
    .record(duration_ms as f64);

// On task failure
counter!("a2a.tasks.failed",
    "tool" => task.tool.clone(),
    "reason" => "error"
).increment(1);

// On task timeout
counter!("a2a.tasks.failed",
    "tool" => task.tool.clone(),
    "reason" => "timeout"
).increment(1);

// Tasks claimed per cycle
gauge!("a2a.processor.tasks_claimed").set(tasks.len() as f64);
```

## Prometheus Configuration

### Scrape Config

```yaml
scrape_configs:
  - job_name: 'api-gateway'
    static_configs:
      - targets: ['api-gateway:8080']
    metrics_path: /metrics
    scrape_interval: 15s
```

### Recording Rules

```yaml
groups:
  - name: a2a_tasks
    rules:
      # Success rate over 5 minutes
      - record: a2a:tasks:success_rate_5m
        expr: |
          sum(rate(a2a_tasks_completed_total[5m])) /
          sum(rate(a2a_tasks_started_total[5m]))

      # Average duration per tool
      - record: a2a:tasks:avg_duration_ms
        expr: |
          sum by (tool) (rate(a2a_tasks_duration_ms_sum[5m])) /
          sum by (tool) (rate(a2a_tasks_duration_ms_count[5m]))

      # Timeout rate
      - record: a2a:tasks:timeout_rate_5m
        expr: |
          sum(rate(a2a_tasks_failed_total{reason="timeout"}[5m])) /
          sum(rate(a2a_tasks_started_total[5m]))
```

## Alerting Rules

### High Failure Rate

```yaml
groups:
  - name: a2a_alerts
    rules:
      - alert: A2AHighFailureRate
        expr: |
          sum(rate(a2a_tasks_failed_total[15m])) /
          sum(rate(a2a_tasks_started_total[15m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "A2A task failure rate above 10%"
          description: "{{ $value | humanizePercentage }} of tasks failing"

      - alert: A2AHighTimeoutRate
        expr: |
          sum(rate(a2a_tasks_failed_total{reason="timeout"}[15m])) /
          sum(rate(a2a_tasks_started_total[15m])) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "A2A task timeout rate above 5%"

      - alert: A2ASlowProcessing
        expr: |
          histogram_quantile(0.95,
            sum by (le) (rate(a2a_tasks_duration_ms_bucket[5m]))
          ) > 25000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "A2A task p95 latency above 25 seconds"
```

## Grafana Dashboard

### Recommended Panels

1. **Task Volume**
   - Total tasks per minute
   - Breakdown by tool
   - Comparison with previous period

2. **Success Rate**
   - Overall success rate gauge
   - Per-tool success rate
   - Historical trend

3. **Duration Percentiles**
   - p50, p95, p99 latencies
   - Per-tool breakdown
   - Heat map visualization

4. **Error Analysis**
   - Error vs timeout breakdown
   - Top failing tools
   - Error rate trend

### Example Dashboard JSON

```json
{
  "panels": [
    {
      "title": "Task Volume",
      "type": "timeseries",
      "targets": [
        {
          "expr": "sum(rate(a2a_tasks_started_total[1m])) * 60",
          "legendFormat": "Tasks/min"
        }
      ]
    },
    {
      "title": "Success Rate",
      "type": "gauge",
      "targets": [
        {
          "expr": "sum(rate(a2a_tasks_completed_total[5m])) / sum(rate(a2a_tasks_started_total[5m]))",
          "legendFormat": "Success Rate"
        }
      ],
      "fieldConfig": {
        "defaults": {
          "unit": "percentunit",
          "min": 0,
          "max": 1
        }
      }
    }
  ]
}
```

## SLO Recommendations

| Metric | Target | Threshold |
|--------|--------|-----------|
| Success Rate | 99.5% | Alert at < 99% |
| P95 Latency | < 20s | Alert at > 25s |
| Timeout Rate | < 1% | Alert at > 5% |
| Availability | 99.9% | Alert at < 99.5% |

## Troubleshooting with Metrics

### High Failure Rate

1. Check `a2a_tasks_failed_total` by `reason`
2. If `timeout` is high, check database performance
3. If `error` is high, check audit logs for error messages

### Slow Processing

1. Check `a2a_tasks_duration_ms` histogram buckets
2. Identify slow tools from label breakdown
3. Check database query performance

### No Tasks Processing

1. Check `a2a_processor_tasks_claimed` gauge
2. Verify task processor is running
3. Check for database connectivity issues

## Related Documentation

- [Audit Logging](./AUDIT_LOGGING.md)
- [Troubleshooting](./TROUBLESHOOTING.md)
- [Runbook](./RUNBOOK.md)

---

**Last Updated**: December 21, 2024
