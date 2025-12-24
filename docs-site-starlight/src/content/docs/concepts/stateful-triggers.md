---
title: Stateful Triggers
description: Advanced triggers with EMA and Rate Counter conditions
sidebar:
  order: 4
---

Stateful triggers maintain state between evaluations, enabling advanced monitoring patterns like trend detection and spam prevention.

## Overview

Regular triggers evaluate each event independently. Stateful triggers remember past events to detect patterns over time.

| Type | Use Case | Example |
|------|----------|---------|
| EMA | Trend detection | Gradual reputation decline |
| Rate Counter | Frequency limiting | Spam detection |

## Exponential Moving Average (EMA)

EMA smooths score trends to detect gradual changes that individual events might miss.

### How EMA Works

```
EMA = α × current_value + (1 - α) × previous_EMA

where α = 2 / (window_size + 1)
```

A larger window size creates a smoother average that's less reactive to individual spikes.

### Configuration

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/conditions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "condition_type": "ema_threshold",
    "field": "score",
    "operator": "<",
    "value": "70",
    "config": {
      "window_size": 10
    }
  }'
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `field` | string | Field to track (e.g., `score`) |
| `operator` | string | Comparison: `<`, `>`, `<=`, `>=`, `=`, `!=` |
| `value` | string | Threshold value |
| `config.window_size` | number | Smoothing factor (default: 10) |

### Example: Reputation Decline Alert

Detect when an agent's reputation trends below 60:

```json
{
  "name": "Reputation Decline Alert",
  "conditions": [
    {
      "condition_type": "event_type_equals",
      "value": "ReputationUpdated"
    },
    {
      "condition_type": "ema_threshold",
      "field": "score",
      "operator": "<",
      "value": "60",
      "config": {
        "window_size": 5
      }
    }
  ]
}
```

With `window_size: 5`, the EMA reacts relatively quickly to changes. Use larger values (10-20) for more stability.

## Rate Counter

Rate counters track event frequency within a sliding time window.

### How Rate Counter Works

```
Events in window → Count → Compare to threshold

Window slides forward, old events are pruned
```

### Configuration

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/conditions" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "condition_type": "rate_limit",
    "field": "event_count",
    "operator": ">",
    "value": "10",
    "config": {
      "time_window": "1h",
      "reset_on_trigger": false
    }
  }'
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `operator` | string | Comparison: `<`, `>`, `<=`, `>=`, `=`, `!=` |
| `value` | string | Count threshold |
| `config.time_window` | string | Window size: `30s`, `5m`, `1h`, `24h` |
| `config.reset_on_trigger` | boolean | Reset count when triggered |

### Time Window Format

| Unit | Format | Example |
|------|--------|---------|
| Seconds | `Ns` | `30s` |
| Minutes | `Nm` | `5m` |
| Hours | `Nh` | `1h` |
| Days | `Nd` | `7d` |

### Example: Spam Detection

Alert when an agent receives more than 20 feedbacks in 1 hour:

```json
{
  "name": "Spam Alert",
  "conditions": [
    {
      "condition_type": "event_type_equals",
      "value": "FeedbackSubmitted"
    },
    {
      "condition_type": "agent_id_equals",
      "value": "42"
    },
    {
      "condition_type": "rate_limit",
      "field": "event_count",
      "operator": ">",
      "value": "20",
      "config": {
        "time_window": "1h",
        "reset_on_trigger": true
      }
    }
  ]
}
```

With `reset_on_trigger: true`, the counter resets after each alert, so you only get notified once per burst.

## Combining Conditions

Stateful and stateless conditions can be combined. All conditions must match for the trigger to fire.

### Example: High-Volume Reputation Drop

```json
{
  "name": "High-Volume Reputation Drop",
  "conditions": [
    {
      "condition_type": "event_type_equals",
      "value": "ReputationUpdated"
    },
    {
      "condition_type": "ema_threshold",
      "field": "score",
      "operator": "<",
      "value": "50",
      "config": {
        "window_size": 10
      }
    },
    {
      "condition_type": "rate_limit",
      "field": "event_count",
      "operator": ">",
      "value": "5",
      "config": {
        "time_window": "30m"
      }
    }
  ]
}
```

This trigger fires when:
1. A reputation update occurs
2. The EMA of scores is below 50
3. More than 5 events occurred in the last 30 minutes

## State Management

### How State Is Stored

Trigger state is stored in PostgreSQL as JSONB:

```json
{
  "ema": {
    "score": {
      "value": 65.5,
      "count": 12
    }
  },
  "rate_counter": {
    "event_count": {
      "timestamps": [1705312800, 1705312900, ...]
    }
  }
}
```

### State Lifecycle

1. **Load** - State loaded before evaluation
2. **Evaluate** - Conditions checked against current state
3. **Update** - State updated with new event data
4. **Persist** - State saved atomically to database

### Memory Limits

Rate counters have a maximum of 10,000 timestamps per window. Older timestamps are automatically pruned.

## Performance

Stateful triggers add minimal overhead:

| Operation | Typical Latency |
|-----------|-----------------|
| State load | 1-2ms |
| EMA calculation | < 1ms |
| Rate counter check | < 1ms |
| State persist | 2-3ms |

**Total overhead**: < 10ms per stateful condition

## Best Practices

1. **Choose appropriate windows** - Match window size to your use case
2. **Use reset wisely** - `reset_on_trigger` prevents alert storms
3. **Combine with filters** - Add `agent_id` conditions to scope state per agent
4. **Monitor state size** - Large rate counter windows consume more memory
5. **Test thresholds** - Start with conservative values and adjust

## Debugging

View trigger state:

```bash
curl "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/state" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

Response:
```json
{
  "trigger_id": "trig_abc123",
  "state": {
    "ema": {
      "score": {"value": 72.3, "count": 25}
    },
    "rate_counter": {
      "event_count": {"count": 8}
    }
  },
  "last_updated": "2024-01-15T10:00:00Z"
}
```
