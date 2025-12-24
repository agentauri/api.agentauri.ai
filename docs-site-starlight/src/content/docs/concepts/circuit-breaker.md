---
title: Circuit Breaker
description: Automatic trigger protection against cascading failures
sidebar:
  order: 5
---

The Circuit Breaker pattern automatically disables failing triggers to prevent cascading failures and protect your systems.

## Overview

When a trigger's actions repeatedly fail, the circuit breaker opens to stop further executions. This prevents:

- Hammering failing endpoints
- Wasting credits on doomed requests
- Alert storms to your notification channels
- Cascading failures across your infrastructure

## States

```
     ┌──────────┐
     │  CLOSED  │ ← Normal operation
     └────┬─────┘
          │ failures > threshold
          ▼
     ┌──────────┐
     │   OPEN   │ ← Blocking executions
     └────┬─────┘
          │ timeout expires
          ▼
     ┌──────────┐
     │HALF-OPEN │ ← Testing recovery
     └────┬─────┘
          │
    ┌─────┴─────┐
    ▼           ▼
 success     failure
    │           │
    ▼           ▼
 CLOSED       OPEN
```

### CLOSED (Normal)

- Actions execute normally
- Failures are counted
- Opens after threshold exceeded

### OPEN (Blocking)

- Actions are skipped
- Events logged but not processed
- Waits for timeout before testing

### HALF-OPEN (Testing)

- Allows one test execution
- Success → CLOSED (reset)
- Failure → OPEN (restart timeout)

## Configuration

Circuit breaker settings are per-trigger:

```bash
curl -X PUT "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "circuit_breaker": {
      "failure_threshold": 5,
      "timeout_seconds": 60,
      "half_open_max_calls": 1
    }
  }'
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `failure_threshold` | 5 | Consecutive failures to open circuit |
| `timeout_seconds` | 60 | Time before testing recovery |
| `half_open_max_calls` | 1 | Test calls allowed in half-open state |

## Monitoring

### Trigger Status

Check a trigger's circuit breaker state:

```bash
curl "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

Response:
```json
{
  "id": "trig_abc123",
  "name": "Agent Alert",
  "enabled": true,
  "circuit_breaker_state": "open",
  "circuit_breaker_opened_at": "2024-01-15T10:00:00Z",
  "consecutive_failures": 5,
  "last_failure_reason": "HTTP 503 Service Unavailable"
}
```

### List Open Circuits

Find all triggers with open circuit breakers:

```bash
curl "https://api.agentauri.ai/api/v1/triggers?circuit_state=open" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Manual Override

### Force Close

Reset the circuit breaker to allow executions:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/circuit-breaker/reset" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Force Open

Manually open the circuit to stop executions:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/triggers/TRIGGER_ID/circuit-breaker/open" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Failure Types

These failures count toward the threshold:

| Failure Type | Description |
|--------------|-------------|
| HTTP 4xx | Client errors (except 429) |
| HTTP 5xx | Server errors |
| Timeout | Request exceeded timeout |
| Connection | DNS/TCP/TLS failures |
| Invalid Response | Malformed response body |

### Not Counted as Failures

| Type | Reason |
|------|--------|
| HTTP 429 | Rate limiting (temporary) |
| Condition mismatch | Trigger didn't match event |
| Disabled trigger | Intentionally paused |

## Events

Circuit breaker state changes emit events:

```json
{
  "event_type": "circuit_breaker_opened",
  "trigger_id": "trig_abc123",
  "reason": "5 consecutive failures",
  "last_error": "HTTP 503 Service Unavailable",
  "timestamp": "2024-01-15T10:00:00Z"
}
```

### Event Types

| Event | Description |
|-------|-------------|
| `circuit_breaker_opened` | Circuit opened due to failures |
| `circuit_breaker_half_opened` | Testing recovery |
| `circuit_breaker_closed` | Recovered, back to normal |
| `circuit_breaker_reset` | Manually reset |

## Notifications

Get alerted when circuits open:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/organizations/ORG_ID/alerts" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "type": "circuit_breaker",
    "notify_email": true,
    "notify_webhook": "https://your-server.com/alerts"
  }'
```

## Best Practices

### 1. Set Appropriate Thresholds

| Use Case | Threshold | Timeout |
|----------|-----------|---------|
| Critical alerts | 3 | 30s |
| Standard monitoring | 5 | 60s |
| Batch processing | 10 | 300s |

### 2. Monitor Circuit States

- Set up alerts for open circuits
- Review failure reasons regularly
- Track recovery patterns

### 3. Test Webhook Endpoints

Before going live:
- Verify endpoint accessibility
- Test error handling
- Confirm timeout settings

### 4. Use Gradual Recovery

After fixing issues:
1. Manually reset circuit
2. Monitor first few executions
3. Adjust thresholds if needed

## Metrics

Circuit breaker exposes Prometheus metrics:

```
# Circuit breaker state (0=closed, 1=open, 2=half-open)
agentauri_circuit_breaker_state{trigger_id="trig_abc123"} 0

# Consecutive failures
agentauri_circuit_breaker_failures{trigger_id="trig_abc123"} 0

# Total state transitions
agentauri_circuit_breaker_transitions_total{trigger_id="trig_abc123",from="closed",to="open"} 5
```

## Troubleshooting

### Circuit Opens Too Often

- Increase `failure_threshold`
- Check webhook endpoint reliability
- Review network connectivity
- Verify API credentials

### Circuit Stays Open

- Check if endpoint is actually recovered
- Verify timeout has elapsed
- Manually reset to test
- Review half-open test results

### False Positives

- Ensure timeout is appropriate
- Check for intermittent network issues
- Review which errors count as failures
