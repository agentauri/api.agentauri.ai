# System Overview

## Introduction

The api.agentauri.ai backend infrastructure provides real-time monitoring and automated response capabilities for the ERC-8004 on-chain agent economy. It bridges blockchain events with off-chain systems through a programmable trigger engine.

## Core Concepts

### ERC-8004 Registries

The system monitors three standardized on-chain registries:

1. **Identity Registry** (ERC-721)
   - Establishes agent identity on-chain
   - Stores agent metadata and endpoint declarations
   - Emits: `Registered`, `MetadataSet` events

2. **Reputation Registry**
   - Tracks agent performance through client feedback
   - Score range: 0-100 with optional semantic tags
   - Emits: `NewFeedback`, `FeedbackRevoked`, `ResponseAppended` events

3. **Validation Registry**
   - Third-party verification of agent capabilities
   - Enables trusted validator ecosystem
   - Emits: `ValidationRequest`, `ValidationResponse` events

### System Layers

```
┌──────────────────────────────────────────────────────────┐
│ Layer 1: Blockchain                                       │
│ - ERC-8004 contracts on multiple chains                   │
│ - Event emission and state changes                        │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│ Layer 2: Indexing                                         │
│ - Ponder indexers (one per chain)                         │
│ - Event normalization and validation                      │
│ - Reorg handling and checkpoint management                │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│ Layer 3: Storage                                          │
│ - Event Store (immutable event log)                       │
│ - Trigger Store (user-defined automation rules)           │
│ - State Store (stateful trigger memory)                   │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│ Layer 4: Processing                                       │
│ - Event Processor (trigger matching engine)               │
│ - State management (EMA, counters, patterns)              │
│ - Rate limiting and circuit breaking                      │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│ Layer 5: Execution                                        │
│ - Action Workers (Telegram, REST, MCP)                    │
│ - Retry logic and error handling                          │
│ - Result logging and metrics                              │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│ Layer 6: Output                                           │
│ - Telegram notifications                                  │
│ - Webhook calls                                           │
│ - Agent MCP server updates                                │
└──────────────────────────────────────────────────────────┘
```

## Key Features

### Multi-Chain Support

The system is designed for horizontal scalability across multiple blockchain networks:

- **Independent indexers**: Each chain has its own Ponder indexer
- **Shared infrastructure**: Common Event Store, Trigger Store, and Action Workers
- **Unified API**: Single API Gateway for managing triggers across all chains

**Supported Networks** (Initial):
- Ethereum Sepolia (testnet)
- Base Sepolia (testnet)
- Linea Sepolia (testnet)
- Polygon Amoy (testnet)

### Programmable Triggers

Users define automation rules consisting of:

**Conditions** (event matching):
- Simple: `score < 60`, `agent_id = 42`, `tag1 = "trade"`
- Complex: `EMA(score, 10) < 70`, `count(feedback) > 10/hour`
- Hybrid: OASF metadata matching, registration file parsing

**Actions** (automated responses):
- Telegram notifications with custom templates
- REST API webhooks with configurable payloads
- MCP server updates (agent feedback push)

### Stateful Processing

The system maintains state for complex trigger conditions:

- **Exponential Moving Average (EMA)**: Track score trends over time
- **Rate Counters**: Detect spam or anomalous activity patterns
- **Time Windows**: Aggregate events within sliding windows
- **Pattern Matching**: Identify sequences or correlations

### Reliability Features

- **Event Store**: Immutable audit trail for all blockchain events
- **Checkpoint System**: Recover from failures without missing events
- **Retry Logic**: Exponential backoff for transient failures
- **Dead Letter Queue**: Capture permanently failed actions for review
- **Circuit Breaker**: Auto-disable problematic triggers
- **Rate Limiting**: Prevent spam and cost overruns

## Data Flow

### Event Ingestion Flow

```
1. Transaction submitted to ERC-8004 contract
   ↓
2. RPC node receives block with transaction
   ↓
3. Ponder indexer detects event via WebSocket/polling
   ↓
4. Event normalized to common schema
   ↓
5. Event written to PostgreSQL Event Store
   ↓
6. PostgreSQL NOTIFY sent on 'new_event' channel
```

### Trigger Evaluation Flow

```
1. Event Processor receives NOTIFY
   ↓
2. Load relevant triggers from Trigger Store
   (filtered by chain_id + registry + enabled)
   ↓
3. For each trigger:
   a. Evaluate all conditions
   b. Update stateful trigger state (if applicable)
   c. Check rate limits and circuit breaker
   ↓
4. If trigger matches:
   a. Create job for each action
   b. Assign priority
   c. Enqueue to Redis
   ↓
5. Commit state changes to database
```

### Action Execution Flow

```
1. Action Worker consumes job from Redis queue
   ↓
2. Execute action based on type:
   - Telegram: Send formatted message
   - REST: Make HTTP request
   - MCP: Push to agent server
   ↓
3. Handle response:
   - Success: Log result
   - Transient error: Retry with backoff
   - Permanent error: Move to DLQ
   ↓
4. Write action result to Result Logger
   ↓
5. Update metrics (Prometheus)
```

## Scalability

### Horizontal Scaling Points

1. **Ponder Indexers**: Deploy separate instances for each chain
2. **Action Workers**: Scale worker pools independently by type
3. **API Gateway**: Load balance across multiple instances
4. **PostgreSQL**: Read replicas for query scaling
5. **Redis**: Cluster mode for queue throughput

### Performance Characteristics

| Component | Throughput | Latency (p95) |
|-----------|-----------|---------------|
| Event Ingestion | 1000+ events/sec | < 500ms |
| Trigger Matching | 500+ evals/sec | < 100ms |
| Action Execution (Telegram) | 30 messages/sec | < 2s |
| Action Execution (REST) | 100+ requests/sec | < 5s |
| Action Execution (MCP) | 50+ calls/sec | < 10s |

## Security Model

### Authentication

- **API Gateway**: JWT-based authentication with refresh tokens
- **User Management**: Username/password or wallet-based sign-in
- **Resource Ownership**: Users can only manage their own triggers

### Authorization

- **Trigger Ownership**: Enforce user_id matching on all CRUD operations
- **Rate Limiting**: Per-user API limits (100 req/min)
- **Action Validation**: Verify destinations (chat IDs, webhook URLs) belong to user

### Data Protection

- **Encryption in Transit**: TLS for all external communication
- **Encryption at Rest**: Database encryption for sensitive data
- **Secrets Management**: Environment variables or dedicated secret manager
- **Input Sanitization**: Validate and sanitize all user inputs

## Observability

### Metrics (Prometheus)

- Request rate, latency, error rate per API endpoint
- Event processing rate per chain and registry
- Action execution success/failure rates by type
- Queue depth and lag
- Database connection pool utilization

### Logging (Structured JSON)

- All HTTP requests/responses with correlation IDs
- Trigger evaluations with match/no-match outcomes
- Action executions with full context
- Errors with stack traces and relevant metadata

### Tracing (Jaeger/Tempo)

- Distributed traces from event → trigger match → action execution
- Performance bottleneck identification
- Cross-service dependency visualization

### Alerting

- Error rate >5% for any component
- Action execution latency exceeds SLO
- Queue depth >10,000 jobs
- Database connection pool exhaustion
- RPC provider failures

## Deployment Architecture

### Development

```
┌─────────────────────────────────────────┐
│ Docker Compose                          │
│                                         │
│  ┌──────────┐  ┌──────────┐            │
│  │PostgreSQL│  │  Redis   │            │
│  └──────────┘  └──────────┘            │
│                                         │
│  ┌──────────┐  ┌──────────┐            │
│  │  Ponder  │  │   API    │            │
│  │ Indexers │  │ Gateway  │            │
│  └──────────┘  └──────────┘            │
│                                         │
│  ┌──────────────────────────┐          │
│  │    Action Workers        │          │
│  │ (Telegram, REST, MCP)    │          │
│  └──────────────────────────┘          │
└─────────────────────────────────────────┘
```

### Production

```
┌─────────────────────────────────────────────────────┐
│ Cloud Infrastructure (AWS/GCP/Azure)                │
│                                                     │
│  ┌─────────────────┐   ┌─────────────────┐         │
│  │ RDS PostgreSQL  │   │ ElastiCache     │         │
│  │ (Multi-AZ)      │   │ Redis (Cluster) │         │
│  └─────────────────┘   └─────────────────┘         │
│                                                     │
│  ┌─────────────────────────────────────────┐       │
│  │ Kubernetes Cluster / ECS                │       │
│  │                                         │       │
│  │  ┌──────────┐ ┌──────────┐             │       │
│  │  │ Ponder   │ │ Ponder   │ (per chain) │       │
│  │  │ Pod 1    │ │ Pod 2    │             │       │
│  │  └──────────┘ └──────────┘             │       │
│  │                                         │       │
│  │  ┌───────────────────────┐              │       │
│  │  │ API Gateway Pods      │              │       │
│  │  │ (Load Balanced)       │              │       │
│  │  └───────────────────────┘              │       │
│  │                                         │       │
│  │  ┌───────────────────────┐              │       │
│  │  │ Event Processor Pods  │              │       │
│  │  └───────────────────────┘              │       │
│  │                                         │       │
│  │  ┌───────────────────────┐              │       │
│  │  │ Action Worker Pods    │              │       │
│  │  │ (by type: TG/REST/MCP)│              │       │
│  │  └───────────────────────┘              │       │
│  └─────────────────────────────────────────┘       │
│                                                     │
│  ┌─────────────────────────────────────────┐       │
│  │ Observability Stack                     │       │
│  │  - Prometheus (metrics)                 │       │
│  │  - Grafana (dashboards)                 │       │
│  │  - Loki (logs)                          │       │
│  │  - Tempo (traces)                       │       │
│  └─────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────┘
```

## Technology Choices

### Why Rust for Backend Services?

- **Performance**: Low latency trigger matching and action execution
- **Safety**: Memory safety and thread safety prevent entire classes of bugs
- **Concurrency**: Tokio async runtime handles thousands of concurrent operations
- **Ecosystem**: Excellent libraries (Actix-web, SQLx, Reqwest)

### Why Ponder for Indexing?

- **Purpose-Built**: Designed specifically for blockchain indexing
- **Reorg Handling**: Automatic chain reorganization detection and correction
- **Type Safety**: Viem integration provides end-to-end type safety
- **Developer Experience**: Hot reload, built-in GraphQL API, easy debugging

### Why PostgreSQL + TimescaleDB?

- **Reliability**: ACID guarantees for critical trigger and event data
- **Time-Series**: TimescaleDB optimizes event storage and queries
- **NOTIFY/LISTEN**: Real-time event notifications without polling
- **JSONB**: Flexible schema for condition configs and action payloads

### Why Redis for Queuing?

- **Performance**: In-memory speed for job queueing
- **Reliability**: Persistence options for durability
- **Features**: Priority queues, TTL, pub/sub
- **Simplicity**: Easy to deploy and operate

## Next Steps

For detailed information on specific components, see:

- [Component Diagrams](./component-diagrams.md)
- [Data Flow](./data-flow.md)
- [Deployment Architecture](./deployment-architecture.md)

For integration details, see:
- [ERC-8004 Integration](../protocols/erc-8004-integration.md)
- [MCP Integration](../protocols/mcp-integration.md)
- [OASF Schema](../protocols/oasf-schema.md)
