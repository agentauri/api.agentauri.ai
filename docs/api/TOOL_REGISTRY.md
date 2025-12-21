# Tool Registry

> **Status**: Implemented
> **Last Updated**: December 2024

This document describes the centralized Tool Registry for A2A Protocol tools, including available tools, tiers, costs, and usage guidelines.

## Overview

The Tool Registry provides centralized management of all A2A Protocol query tools. It defines:

- Available tools and their capabilities
- Tool tiers based on complexity
- Pricing in micro-USDC
- Validation rules

## Tool Tiers

Tools are organized into tiers based on query complexity and computational cost:

| Tier | Name | Description | Cost Range |
|------|------|-------------|------------|
| **Tier 0** | Raw Data | Direct database queries, lowest latency | 0.001 USDC (1,000 micro-USDC) |
| **Tier 1** | Aggregated | Computed aggregations, moderate cost | 0.01 USDC (10,000 micro-USDC) |
| **Tier 2** | Analysis | Complex analysis (future) | TBD |
| **Tier 3** | AI-Powered | LLM-enhanced analysis, highest cost | 0.20 USDC (200,000 micro-USDC) |

## Available Tools

### Tier 0: Raw Data

#### getMyFeedbacks

Get all feedback records for a specific agent.

**Cost**: 0.001 USDC

**Parameters**:
```json
{
  "agentId": 42
}
```

**Response**:
```json
{
  "feedbacks": [
    {
      "id": "uuid",
      "score": 85,
      "comment": "Great performance",
      "validator_id": 123,
      "created_at": "2024-11-15T10:30:00Z"
    }
  ],
  "total": 127
}
```

---

#### getAgentProfile

Get agent profile and metadata from the Identity Registry.

**Cost**: 0.001 USDC

**Parameters**:
```json
{
  "agentId": 42
}
```

**Response**:
```json
{
  "agent_id": 42,
  "name": "Agent Smith",
  "owner": "0x1234...abcd",
  "metadata_uri": "ipfs://...",
  "created_at": "2024-01-15T00:00:00Z",
  "is_active": true
}
```

---

### Tier 1: Aggregated

#### getReputationSummary

Get aggregated reputation statistics for an agent.

**Cost**: 0.01 USDC

**Parameters**:
```json
{
  "agentId": 42
}
```

**Response**:
```json
{
  "agent_id": 42,
  "reputation_score": 0.85,
  "total_feedbacks": 127,
  "positive_feedbacks": 112,
  "negative_feedbacks": 15,
  "neutral_feedbacks": 0,
  "average_score": 87.5,
  "computed_at": "2024-11-15T10:30:00Z"
}
```

---

#### getTrend

Get reputation trend over time for an agent.

**Cost**: 0.01 USDC

**Parameters**:
```json
{
  "agentId": 42,
  "period": "30d"
}
```

**Response**:
```json
{
  "agent_id": 42,
  "period": "30d",
  "data_points": [
    {"date": "2024-11-01", "score": 0.82},
    {"date": "2024-11-08", "score": 0.84},
    {"date": "2024-11-15", "score": 0.85}
  ],
  "trend": "improving",
  "change_percent": 3.66
}
```

---

#### getValidationHistory

Get validation history for an agent.

**Cost**: 0.01 USDC

**Parameters**:
```json
{
  "agentId": 42
}
```

**Response**:
```json
{
  "agent_id": 42,
  "validations": [
    {
      "id": "uuid",
      "validator_id": 123,
      "result": "passed",
      "score": 92,
      "timestamp": "2024-11-15T10:30:00Z"
    }
  ],
  "total": 45,
  "pass_rate": 0.93
}
```

---

### Tier 3: AI-Powered

#### getReputationReport

Get an AI-generated comprehensive reputation analysis report.

**Cost**: 0.20 USDC

**Parameters**:
```json
{
  "agentId": 42,
  "includeRecommendations": true
}
```

**Response**:
```json
{
  "agent_id": 42,
  "summary": "Agent demonstrates consistent high performance...",
  "strengths": [
    "Reliable response times",
    "High accuracy scores"
  ],
  "weaknesses": [
    "Occasional timeout issues under high load"
  ],
  "recommendations": [
    "Consider implementing retry logic",
    "Add monitoring for edge cases"
  ],
  "overall_rating": "A",
  "confidence": 0.92,
  "generated_at": "2024-11-15T10:30:00Z"
}
```

## Credit System

### Micro-USDC

All costs are tracked in micro-USDC where:
- 1 USDC = 1,000,000 micro-USDC
- Tier 0: 1,000 micro-USDC
- Tier 1: 10,000 micro-USDC
- Tier 3: 200,000 micro-USDC

### Credit Validation

Before a task is created, the system validates:
1. Organization has a credit balance initialized
2. Balance is sufficient for the tool's cost
3. If insufficient, returns error `-32004`

### Credit Deduction

Credits are deducted only after successful task completion. Failed or cancelled tasks do not consume credits.

## Validation Rules

### Tool Validation

- Tool name must match exactly (case-sensitive)
- Invalid tools return error `-32602` with list of valid tools

### Argument Validation

- Arguments must be valid JSON
- Maximum payload size: 100KB
- Required parameters validated per tool

## Implementation

The Tool Registry is implemented in `rust-backend/crates/api-gateway/src/services/tool_registry.rs`.

### Key Functions

```rust
// Check if tool exists and is enabled
ToolRegistry::is_valid("getReputationSummary") // true

// Get cost in micro-USDC
ToolRegistry::get_cost_micro_usdc("getReputationSummary") // 10_000

// Get human-readable cost
ToolRegistry::get_cost_display("getReputationSummary") // "0.01 USDC"

// Get tool tier
ToolRegistry::get_tier("getReputationSummary") // Some(ToolTier::Tier1)

// List all enabled tools
ToolRegistry::list_enabled()

// List tools by tier
ToolRegistry::list_by_tier(ToolTier::Tier0)
```

## Adding New Tools

To add a new tool:

1. Add entry to `TOOL_REGISTRY` in `tool_registry.rs`:
```rust
tools.insert(
    "newToolName",
    ToolDefinition {
        name: "newToolName",
        tier: ToolTier::Tier1,
        cost_micro_usdc: 10_000,
        cost_display: "0.01 USDC",
        description: "Description of the new tool",
        enabled: true,
    },
);
```

2. Implement the query logic in `query_executor.rs`

3. Add tests for the new tool

4. Update this documentation

## Related Documentation

- [A2A Protocol Integration](../protocols/A2A_INTEGRATION.md)
- [API Documentation](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)
- [Query Tools](./QUERY_TOOLS.md)

---

**Last Updated**: December 21, 2024
