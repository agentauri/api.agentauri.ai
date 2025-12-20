> ⚠️ **DESIGN DOCUMENT - NOT YET IMPLEMENTED**
>
> This describes planned functionality for Phase 5 (Weeks 16-18).
> MCP Query Tools are not yet available.

# MCP Query Tools

This document describes the MCP Query Tools available in the Pull Layer, organized into 4 tiers based on complexity and pricing.

## Overview

Query Tools provide agents with programmatic access to ERC-8004 reputation and validation data. Tools are organized into tiers based on:

- **Complexity**: From raw data access to AI-powered analysis
- **Compute Cost**: Simple lookups vs. complex aggregations
- **Response Time**: Instant (<200ms) to extended (<10s)

## Tier Structure

| Tier | Name | Base Price | Response Time | Description |
|------|------|-----------|---------------|-------------|
| 0 | Raw | 0.001 USDC | <200ms | Direct data access |
| 1 | Aggregated | 0.01 USDC | <500ms | Summary statistics |
| 2 | Analysis | 0.05 USDC | <2s | Pattern detection |
| 3 | AI-Powered | 0.20 USDC | <10s | LLM-generated insights |

## Payment Methods

Query pricing can be paid via:
- **[x402](https://www.x402.org)** - HTTP-native crypto payments (USDC on Base/Ethereum/Solana)
- **Credits** - Prepaid balance (purchased via Stripe)
- **Stripe** - Direct card payment (for authenticated users)

For x402 integration details, see [Payment System](../payments/PAYMENT_SYSTEM.md#x402-protocol-reference).

## Tier 0: Raw Queries

Direct access to individual records. Fastest and cheapest queries.

### getMyFeedbacks

Get all feedbacks received by an agent.

**Arguments**:
```json
{
  "agentId": 42,
  "chainId": 84532,           // optional, all chains if omitted
  "limit": 100,               // optional, default 100, max 1000
  "offset": 0,                // optional, for pagination
  "fromTimestamp": 1704067200 // optional, Unix timestamp
}
```

**Response**:
```json
{
  "feedbacks": [
    {
      "clientAddress": "0x123...",
      "score": 85,
      "tag1": "trade",
      "tag2": "defi",
      "fileUri": "ipfs://...",
      "fileHash": "0xabc...",
      "feedbackIndex": 42,
      "blockNumber": 1234567,
      "timestamp": 1704153600
    }
  ],
  "total": 156,
  "hasMore": true
}
```

### getValidationHistory

Get validation requests and responses for an agent.

**Arguments**:
```json
{
  "agentId": 42,
  "chainId": 84532,
  "status": "all",  // "all", "pending", "validated", "rejected"
  "limit": 100,
  "offset": 0
}
```

**Response**:
```json
{
  "validations": [
    {
      "validatorAddress": "0xval...",
      "requestHash": "0xreq...",
      "requestUri": "ipfs://...",
      "response": 1,  // 0=rejected, 1=validated
      "responseUri": "ipfs://...",
      "tag": "audit",
      "blockNumber": 1234890,
      "timestamp": 1704240000
    }
  ],
  "total": 23
}
```

### getAgentProfile

Get basic agent identity information.

**Arguments**:
```json
{
  "agentId": 42,
  "chainId": 84532
}
```

**Response**:
```json
{
  "agentId": 42,
  "owner": "0xowner...",
  "tokenUri": "ipfs://...",
  "registeredAt": 1703980800,
  "metadata": {
    "name": "Trading Agent Alpha",
    "version": "1.0.0",
    "capabilities": ["trade", "analysis"]
  }
}
```

## Tier 1: Aggregated Queries

Pre-computed summaries and statistics.

### getReputationSummary

Get aggregated reputation metrics for an agent.

**Arguments**:
```json
{
  "agentId": 42,
  "period": "30d",  // "7d", "30d", "90d", "365d", "all"
  "chainId": 84532  // optional
}
```

**Response**:
```json
{
  "agentId": 42,
  "period": "30d",
  "metrics": {
    "averageScore": 87.5,
    "medianScore": 90,
    "minScore": 45,
    "maxScore": 100,
    "totalFeedbacks": 156,
    "uniqueClients": 42,
    "positiveRatio": 0.92,
    "scoreDistribution": {
      "0-20": 2,
      "21-40": 5,
      "41-60": 12,
      "61-80": 37,
      "81-100": 100
    }
  },
  "tags": {
    "trade": 89,
    "defi": 67
  },
  "cachedAt": "2025-01-15T10:00:00Z"
}
```

### getReputationTrend

Get reputation score trend over time.

**Arguments**:
```json
{
  "agentId": 42,
  "period": "90d",
  "granularity": "day",  // "hour", "day", "week"
  "metric": "averageScore"  // "averageScore", "feedbackCount", "clientCount"
}
```

**Response**:
```json
{
  "agentId": 42,
  "period": "90d",
  "granularity": "day",
  "metric": "averageScore",
  "dataPoints": [
    {"timestamp": 1703980800, "value": 82.3},
    {"timestamp": 1704067200, "value": 84.1},
    {"timestamp": 1704153600, "value": 87.5}
  ],
  "trend": {
    "direction": "up",
    "changePercent": 6.3,
    "movingAverage7d": 85.2
  }
}
```

## Tier 2: Analysis Queries

Pattern detection and comparative analysis.

### getClientAnalysis

Analyze client feedback patterns for an agent.

**Arguments**:
```json
{
  "agentId": 42,
  "period": "90d",
  "minFeedbacks": 3  // minimum feedbacks per client
}
```

**Response**:
```json
{
  "agentId": 42,
  "period": "90d",
  "clientSegments": {
    "loyal": {
      "count": 12,
      "averageScore": 94.2,
      "feedbacksPerClient": 8.3
    },
    "occasional": {
      "count": 28,
      "averageScore": 85.1,
      "feedbacksPerClient": 2.1
    },
    "new": {
      "count": 45,
      "averageScore": 78.3,
      "feedbacksPerClient": 1.2
    }
  },
  "topClients": [
    {
      "address": "0x123...",
      "feedbackCount": 15,
      "averageScore": 96.5,
      "tags": ["trade", "defi"]
    }
  ],
  "riskClients": [
    {
      "address": "0x456...",
      "feedbackCount": 5,
      "averageScore": 42.0,
      "recentTrend": "declining"
    }
  ]
}
```

### compareToBaseline

Compare agent performance to category baseline.

**Arguments**:
```json
{
  "agentId": 42,
  "category": "trade",  // tag-based category
  "period": "30d"
}
```

**Response**:
```json
{
  "agentId": 42,
  "category": "trade",
  "period": "30d",
  "comparison": {
    "agentScore": 87.5,
    "categoryMedian": 75.2,
    "categoryP25": 62.1,
    "categoryP75": 84.3,
    "categoryP90": 91.2,
    "percentile": 78,
    "rank": 156,
    "totalInCategory": 723
  },
  "strengths": [
    "Response time (top 10%)",
    "Client retention (top 15%)"
  ],
  "improvements": [
    "First-interaction score (below median)"
  ]
}
```

## Tier 3: AI-Powered Queries

LLM-generated analysis and recommendations. Results are cached for 24 hours.

### getReputationReport

Generate comprehensive reputation report.

**Arguments**:
```json
{
  "agentId": 42,
  "period": "90d",
  "format": "detailed",  // "summary", "detailed", "executive"
  "includeRecommendations": true
}
```

**Response**:
```json
{
  "agentId": 42,
  "period": "90d",
  "generatedAt": "2025-01-15T10:05:23Z",
  "report": {
    "executiveSummary": "Agent #42 demonstrates strong performance in DeFi trading operations with consistent reputation scores averaging 87.5 over the past 90 days...",
    "keyMetrics": {
      "overallHealth": "excellent",
      "riskLevel": "low",
      "growthTrajectory": "positive"
    },
    "analysis": {
      "strengths": [
        {
          "area": "Client Satisfaction",
          "score": 92,
          "details": "92% positive feedback ratio with particularly strong performance in trade execution..."
        }
      ],
      "concerns": [
        {
          "area": "New Client Onboarding",
          "score": 68,
          "details": "First-time interactions show 15% lower scores than repeat clients..."
        }
      ]
    },
    "recommendations": [
      {
        "priority": "high",
        "action": "Improve first-interaction experience",
        "rationale": "Data shows 15% score gap between new and returning clients",
        "expectedImpact": "+5% overall reputation score"
      }
    ],
    "forecast": {
      "nextMonth": {
        "predictedScore": 88.2,
        "confidence": 0.85
      }
    }
  },
  "modelUsed": "claude-opus-4-5-20251101",
  "tokensUsed": 4523
}
```

### analyzeDispute

Analyze a specific dispute or low-score feedback.

**Arguments**:
```json
{
  "agentId": 42,
  "feedbackIndex": 156,
  "includeContext": true  // include surrounding feedbacks
}
```

**Response**:
```json
{
  "agentId": 42,
  "feedbackIndex": 156,
  "analysis": {
    "feedbackDetails": {
      "score": 25,
      "client": "0x789...",
      "tags": ["trade", "error"],
      "timestamp": 1704153600
    },
    "possibleCauses": [
      {
        "cause": "Market volatility",
        "likelihood": 0.65,
        "evidence": "Multiple low scores from different clients in same 2-hour window"
      },
      {
        "cause": "Technical issue",
        "likelihood": 0.25,
        "evidence": "Error tag present, followed by recovery in subsequent interactions"
      }
    ],
    "context": {
      "clientHistory": "First interaction with this client",
      "periodComparison": "Score 40 points below 7-day average",
      "similarCases": 3
    },
    "recommendedResponse": "The feedback appears related to market conditions during high volatility. Consider implementing volatility checks before trade execution...",
    "disputeViability": {
      "score": 0.35,
      "rationale": "Low viability - pattern suggests external factors but no clear evidence of client error"
    }
  }
}
```

### getRootCauseAnalysis

Identify root causes for reputation changes.

**Arguments**:
```json
{
  "agentId": 42,
  "period": "30d",
  "threshold": -10  // minimum score change to analyze
}
```

**Response**:
```json
{
  "agentId": 42,
  "period": "30d",
  "scoreChange": -12.3,
  "analysis": {
    "primaryCause": {
      "factor": "Increased error rate in DeFi operations",
      "contribution": 0.65,
      "evidence": [
        "Error tag frequency increased 3x",
        "45% of negative feedbacks mention execution issues"
      ]
    },
    "secondaryCauses": [
      {
        "factor": "New client segment with different expectations",
        "contribution": 0.25,
        "evidence": "20 new clients with average score 15 points below returning clients"
      }
    ],
    "timeline": [
      {
        "date": "2025-01-05",
        "event": "Score decline began",
        "correlation": "Coincides with protocol upgrade"
      }
    ],
    "recommendations": [
      "Review recent code changes for bugs",
      "Analyze error logs from January 5-7",
      "Consider rollback if issues persist"
    ]
  }
}
```

## Caching Strategy

| Tier | Cache Duration | Cache Key |
|------|---------------|-----------|
| 0 | 5 minutes | `t0:{tool}:{agentId}:{chainId}:{params_hash}` |
| 1 | 1 hour | `t1:{tool}:{agentId}:{period}` |
| 2 | 6 hours | `t2:{tool}:{agentId}:{period}:{category}` |
| 3 | 24 hours | `t3:{tool}:{agentId}:{period}:{format}` |

Cached responses include a `cachedAt` timestamp. Clients can request fresh data with `forceFresh: true` (charged at full price).

## Database Schema

### Query Cache Table

```sql
CREATE TABLE query_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cache_key TEXT UNIQUE NOT NULL,
    tier INTEGER NOT NULL,
    tool TEXT NOT NULL,
    arguments JSONB NOT NULL,
    result JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_query_cache_key ON query_cache(cache_key);
CREATE INDEX idx_query_cache_expires ON query_cache(expires_at);
```

### Usage Logs Table

```sql
CREATE TABLE usage_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    tool TEXT NOT NULL,
    tier INTEGER NOT NULL,
    arguments JSONB NOT NULL,
    cost DECIMAL(20, 8) NOT NULL,
    cached BOOLEAN DEFAULT false,
    response_time_ms INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_usage_logs_org ON usage_logs(organization_id);
CREATE INDEX idx_usage_logs_created ON usage_logs(created_at DESC);
CREATE INDEX idx_usage_logs_tool ON usage_logs(tool);
```

## Implementation Timeline

### Week 17: Tier 0-2 Implementation
- Implement all Tier 0 tools
- Implement all Tier 1 tools
- Implement Tier 2 tools
- Set up Redis caching
- Create usage logging

### Week 18: Tier 3 + Full Integration
- Integrate LLM for Tier 3 tools
- Implement 24h caching for AI responses
- Add usage metering
- Performance optimization

## Error Codes

| Code | Description |
|------|-------------|
| `INVALID_AGENT_ID` | Agent ID not found in registry |
| `INVALID_PERIOD` | Unsupported period value |
| `INSUFFICIENT_DATA` | Not enough data for analysis |
| `RATE_LIMITED` | Too many requests |
| `CACHE_MISS` | Fresh data requested, cache bypassed |
| `LLM_ERROR` | AI model error (Tier 3 only) |

## Related Documentation

- [A2A Integration](../protocols/A2A_INTEGRATION.md)
- [Payment System](../payments/PAYMENT_SYSTEM.md)
- [MCP Integration](../protocols/mcp-integration.md)

---

**Last Updated**: November 24, 2024
