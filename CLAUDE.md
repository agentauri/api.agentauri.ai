# api.8004.dev - ERC-8004 Backend Infrastructure

## Project Overview

This project provides a real-time backend infrastructure for monitoring, interpreting, and reacting to events from the ERC-8004 standard's three on-chain registries: Identity, Reputation, and Validation. It enables programmable triggers that execute automated actions based on blockchain events, creating a bridge between on-chain agent activity and off-chain systems.

### Purpose

The ERC-8004 standard defines the foundation for on-chain agent economy:
- **Who** an agent is (Identity Registry)
- **How** an agent is evaluated (Reputation Registry)
- **How** an agent's work is validated (Validation Registry)

This backend transforms these raw blockchain signals into intelligent actions: notifications, API calls, and updates to agent MCP (Model Context Protocol) servers, enabling agents to learn and adapt based on their on-chain reputation.

### Key Capabilities

- **Multi-chain monitoring** of all three ERC-8004 registries
- **Programmable trigger engine** supporting:
  - Simple event-driven conditions (score thresholds, tag filters, agent ID matching)
  - Complex stateful conditions (moving averages, rate limits, pattern detection)
  - Hybrid conditions (registration file parsing, OASF metadata matching)
- **Flexible action execution**:
  - Telegram notifications
  - REST API webhooks
  - MCP server updates (agent feedback push)
- **Event store** for auditability, analytics, replay, and resilience
- **Native scalability** with independent per-chain indexing and async action execution

### ERC-8004 Protocol Reference

- **Specification**: https://eips.ethereum.org/EIPS/eip-8004
- **Contracts Repository**: https://github.com/erc-8004/erc-8004-contracts
- **OASF Schema**: https://github.com/agntcy/oasf

## Architecture

### System Overview

The system consists of 9 core components organized into distinct layers:

```
┌─────────────────────────────────────────────────────────────────┐
│                         Blockchain Layer                         │
│  (ERC-8004 Contracts on Ethereum, Base, Linea, Polygon, etc.)  │
└────────────────┬────────────────────────────────────────────────┘
                 │ JSON-RPC
┌────────────────▼────────────────────────────────────────────────┐
│                        RPC Node Layer                            │
│              (Alchemy, Infura, QuickNode)                        │
└────────────────┬────────────────────────────────────────────────┘
                 │ WebSocket/Polling
┌────────────────▼────────────────────────────────────────────────┐
│                     Indexing Layer                               │
│         Ponder Indexers (TypeScript) - One per chain            │
│    ┌──────────────┬──────────────┬──────────────┐               │
│    │ Identity     │ Reputation   │ Validation   │               │
│    │ Handler      │ Handler      │ Handler      │               │
│    └──────────────┴──────────────┴──────────────┘               │
└────────────────┬────────────────────────────────────────────────┘
                 │ Normalized Events
┌────────────────▼────────────────────────────────────────────────┐
│                       Storage Layer                              │
│         Event Store (PostgreSQL + TimescaleDB)                   │
│         Trigger Store (PostgreSQL)                               │
└────────────────┬────────────────────────────────────────────────┘
                 │ PostgreSQL NOTIFY
┌────────────────▼────────────────────────────────────────────────┐
│                    Processing Layer                              │
│          Event Processor (Rust/Tokio)                            │
│    - Trigger matching                                            │
│    - State management (EMA, counters)                            │
│    - Rate limiting & circuit breaking                            │
└────────────────┬────────────────────────────────────────────────┘
                 │ Job Enqueue
┌────────────────▼────────────────────────────────────────────────┐
│                      Queue Layer                                 │
│                 Redis (Job Queue)                                │
└────────────────┬────────────────────────────────────────────────┘
                 │ Job Consumption
┌────────────────▼────────────────────────────────────────────────┐
│                    Execution Layer                               │
│           Action Workers (Rust/Tokio)                            │
│    ┌──────────────┬──────────────┬──────────────┐               │
│    │ Telegram     │ REST/HTTP    │ MCP Server   │               │
│    │ Worker       │ Worker       │ Worker       │               │
│    └──────────────┴──────────────┴──────────────┘               │
└────────────────┬────────────────────────────────────────────────┘
                 │ Action Execution
┌────────────────▼────────────────────────────────────────────────┐
│                     Output Layer                                 │
│  Telegram Bot API  │  HTTP REST APIs  │  MCP Protocol Servers   │
└──────────────────────────────────────────────────────────────────┘
                 │
          ┌──────┴───────┐
          │ Result Logger│
          │ (PostgreSQL) │
          └──────────────┘
```

### Component Details

#### 1. API Gateway (Rust/Actix-web)

**Status**: ✅ Week 7 Complete (100%)

REST API server providing trigger management and system queries.

**Implemented Endpoints**:
- Authentication: `/api/v1/auth/register`, `/api/v1/auth/login`
- Triggers: `/api/v1/triggers` (CRUD with pagination)
- Conditions: `/api/v1/triggers/{id}/conditions` (CRUD)
- Actions: `/api/v1/triggers/{id}/actions` (CRUD)
- Health: `/api/v1/health` (system status)

**Security Features**:
- JWT authentication middleware (jsonwebtoken crate)
- Argon2 password hashing (secure, modern algorithm)
- User ownership validation on all trigger operations
- CORS whitelist with environment configuration
- Input validation with validator crate

**Architecture**:
- 3-layer design: Handlers → Repositories → Database
- Repository pattern for clean database access
- DTO pattern for request/response serialization
- Compile-time SQL verification with SQLx
- Pagination support (limit/offset parameters)

**Documentation**: See `rust-backend/crates/api-gateway/API_DOCUMENTATION.md` for complete API reference with examples.

#### 2. RPC Nodes (External Services)

**Responsibility**: Blockchain data access via JSON-RPC protocol.

**Providers**:
- Alchemy (primary)
- Infura (fallback)
- QuickNode (additional fallback)

**Features**:
- Connection pooling for throughput
- Automatic retry with exponential backoff
- Load balancing across multiple providers
- Response caching for frequently accessed data
- Rate limit handling

#### 3. Ponder Indexer Layer (TypeScript)

**Responsibility**: Real-time blockchain event monitoring, normalization, and persistence.

**Technology Stack**:
- Ponder (blockchain indexing framework)
- Viem (Ethereum interactions)
- Node.js runtime

**Architecture**:
- **One indexer per chain** (parallel, independent operation)
- Automatic chain reorganization handling
- Checkpoint-based recovery
- Event normalization to common schema

**Event Handlers**:

```typescript
// Identity Registry
onAgentRegistered(agentId, tokenURI, owner)
onMetadataSet(agentId, key, value)

// Reputation Registry
onNewFeedback(agentId, clientAddress, score, tag1, tag2, fileUri, fileHash)
onFeedbackRevoked(agentId, clientAddress, feedbackIndex)
onResponseAppended(agentId, clientAddress, feedbackIndex, responder, responseUri)

// Validation Registry
onValidationRequest(validatorAddress, agentId, requestUri, requestHash)
onValidationResponse(validatorAddress, agentId, requestHash, response, responseUri, tag)
```

**Supported Chains** (Initial):
- Ethereum Sepolia
- Base Sepolia
- Linea Sepolia
- Polygon Amoy

#### 4. Trigger Store (PostgreSQL)

**Responsibility**: Persistent storage of user-defined trigger configurations.

**Schema**:

```sql
-- Trigger definitions
CREATE TABLE triggers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    chain_id INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    enabled BOOLEAN DEFAULT true,
    is_stateful BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Trigger matching conditions
CREATE TABLE trigger_conditions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    condition_type TEXT NOT NULL,
    field TEXT NOT NULL,
    operator TEXT NOT NULL,
    value TEXT NOT NULL,
    config JSONB,  -- For complex conditions (window_size, alpha, etc.)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Actions to execute when trigger matches
CREATE TABLE trigger_actions (
    id SERIAL PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    action_type TEXT NOT NULL CHECK (action_type IN ('telegram', 'rest', 'mcp')),
    priority INTEGER DEFAULT 1,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- State for stateful triggers (EMA, counters, etc.)
CREATE TABLE trigger_state (
    trigger_id TEXT PRIMARY KEY,
    state_data JSONB NOT NULL,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);
```

**Indexes**:
```sql
CREATE INDEX idx_triggers_user_id ON triggers(user_id);
CREATE INDEX idx_triggers_chain_registry_enabled ON triggers(chain_id, registry, enabled) WHERE enabled = true;
CREATE INDEX idx_trigger_conditions_trigger_id ON trigger_conditions(trigger_id);
CREATE INDEX idx_trigger_actions_trigger_id ON trigger_actions(trigger_id);
```

#### 5. Event Store (PostgreSQL + TimescaleDB)

**Responsibility**: Immutable log of all blockchain events for audit, analytics, and recovery.

**Technology Stack**:
- PostgreSQL 15+
- TimescaleDB extension for time-series optimization
- PostgreSQL NOTIFY/LISTEN for real-time notifications

**Schema**:

```sql
-- Main events table (converted to hypertable)
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    event_type TEXT NOT NULL,

    -- Common fields
    agent_id BIGINT,
    timestamp BIGINT NOT NULL,

    -- Identity Registry fields
    owner TEXT,
    token_uri TEXT,
    metadata_key TEXT,
    metadata_value TEXT,

    -- Reputation Registry fields
    client_address TEXT,
    feedback_index BIGINT,
    score INTEGER,
    tag1 TEXT,
    tag2 TEXT,
    file_uri TEXT,
    file_hash TEXT,

    -- Validation Registry fields
    validator_address TEXT,
    request_hash TEXT,
    response INTEGER,
    response_uri TEXT,
    response_hash TEXT,
    tag TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Convert to TimescaleDB hypertable for efficient time-series operations
SELECT create_hypertable('events', 'created_at');

-- Checkpoint tracking per chain
CREATE TABLE checkpoints (
    chain_id INTEGER PRIMARY KEY,
    last_block_number BIGINT NOT NULL,
    last_block_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Trigger function for NOTIFY on new events
CREATE OR REPLACE FUNCTION notify_new_event()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('new_event', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER events_notify_trigger
    AFTER INSERT ON events
    FOR EACH ROW
    EXECUTE FUNCTION notify_new_event();
```

**Indexes**:
```sql
CREATE INDEX idx_events_chain_id_created_at ON events(chain_id, created_at DESC);
CREATE INDEX idx_events_agent_id ON events(agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_events_registry_type ON events(registry, event_type);
CREATE INDEX idx_events_client_address ON events(client_address) WHERE client_address IS NOT NULL;
```

**Retention Policy**:
```sql
-- Continuous aggregates for long-term analytics
CREATE MATERIALIZED VIEW events_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', created_at) AS hour,
    chain_id,
    registry,
    event_type,
    COUNT(*) as event_count
FROM events
GROUP BY hour, chain_id, registry, event_type;

-- Retention policy (optional, depends on requirements)
SELECT add_retention_policy('events', INTERVAL '365 days');
```

#### 6. Event Processor (Rust/Tokio)

**Responsibility**: Core trigger matching engine that evaluates events against user-defined conditions.

**Technology Stack**:
- Tokio (async runtime)
- SQLx (database access)
- Redis client (job queueing)

**Processing Flow**:

```
1. Listen to PostgreSQL NOTIFY on 'new_event' channel
2. Fetch full event details from Event Store
3. Query Trigger Store for relevant triggers (filtered by chain_id + registry)
4. For each trigger:
   a. Evaluate all conditions (AND logic within trigger)
   b. Update stateful trigger state if needed (EMA, counters, rate windows)
   c. Check rate limits and circuit breaker state
   d. If all conditions match:
      - Enqueue action jobs to Redis (one job per action)
      - Update trigger execution metadata
5. Commit state changes
```

**Condition Types**:

| Condition Type | Description | Example |
|----------------|-------------|---------|
| `agent_id_equals` | Exact agent ID match | `agent_id = 42` |
| `score_threshold` | Score comparison | `score < 60` |
| `tag_equals` | Tag matching | `tag1 = "trade"` |
| `validator_whitelist` | Validator in approved list | `validator IN [0x123, 0x456]` |
| `event_type_equals` | Specific event type | `event_type = "NewFeedback"` |
| `ema_threshold` | Exponential moving average | `EMA(score, 10) < 70` |
| `rate_limit` | Event rate in time window | `count(feedback) > 10/hour` |
| `file_uri_exists` | File URI is present | `file_uri IS NOT NULL` |

**Rate Limiting**:
- Per-trigger execution limits (e.g., max 10 executions/hour)
- Redis-based sliding window counters
- Prevents spam and cost overruns

**Circuit Breaker**:
- Monitors action failure rates per trigger
- Auto-disables triggers with >80% failure rate
- Auto-recovery after configurable timeout (default: 1 hour)

#### 7. Action Workers (Rust/Tokio)

**Responsibility**: Execute actions in response to matched triggers.

**Technology Stack**:
- Tokio (async runtime)
- Reqwest (HTTP client for REST and MCP)
- Teloxide (Telegram bot SDK)
- TypeScript MCP SDK (bridged from Rust)

**Worker Types**:

##### Telegram Worker

Sends formatted notifications via Telegram Bot API.

**Features**:
- Message template support with variable substitution
- Markdown/HTML formatting
- Automatic retry on rate limits
- Message chunking for long content

**Configuration Example**:
```json
{
  "action_type": "telegram",
  "config": {
    "chat_id": "123456789",
    "message_template": "⚠️ Agent #{{agent_id}} received low score: {{score}}/100\nFrom: {{client_address}}\nBlock: {{block_number}}",
    "parse_mode": "Markdown"
  }
}
```

##### REST/HTTP Worker

Executes HTTP requests to external APIs.

**Features**:
- Support for GET, POST, PUT, DELETE, PATCH methods
- Custom headers (auth tokens, content-type)
- Request body templating
- Response validation
- Timeout configuration (default: 30s)

**Configuration Example**:
```json
{
  "action_type": "rest",
  "config": {
    "method": "POST",
    "url": "https://api.example.com/webhooks/feedback",
    "headers": {
      "Authorization": "Bearer xxx",
      "Content-Type": "application/json"
    },
    "body_template": {
      "agent_id": "{{agent_id}}",
      "score": "{{score}}",
      "chain_id": "{{chain_id}}"
    },
    "timeout_ms": 30000
  }
}
```

##### MCP Server Worker

Pushes updates to agent MCP servers using the Model Context Protocol.

**Features**:
- Automatic endpoint resolution from registration file (tokenURI)
- File hash verification before sending
- IPFS content fetching and validation
- OASF schema validation
- MCP authentication handling

**Implementation Approach**:
- Use TypeScript MCP SDK (@modelcontextprotocol/sdk)
- Bridge from Rust via HTTP subprocess or embedded runtime
- Cache agent endpoint configurations

**Configuration Example**:
```json
{
  "action_type": "mcp",
  "config": {
    "resolve_endpoint": true,
    "tool_name": "agent.receiveFeedback",
    "verify_file_hash": true,
    "include_file_content": true,
    "validate_oasf": true,
    "payload_template": {
      "score": "{{score}}",
      "tag1": "{{tag1}}",
      "tag2": "{{tag2}}",
      "clientAddress": "{{client_address}}",
      "feedbackIndex": "{{feedback_index}}",
      "fileUri": "{{file_uri}}",
      "fileHash": "{{file_hash}}",
      "fileContent": "{{verified_file_content}}",
      "blockNumber": "{{block_number}}"
    }
  }
}
```

**Common Worker Features**:
- Exponential backoff retry (3 attempts: 1s, 2s, 4s)
- Dead Letter Queue (DLQ) for permanent failures
- Result logging to PostgreSQL
- Prometheus metrics for observability

#### 8. Result Logger (PostgreSQL)

**Responsibility**: Audit trail of all action executions.

**Schema**:

```sql
CREATE TABLE action_results (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    trigger_id TEXT,
    event_id TEXT,
    action_type TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('success', 'failed', 'retrying')),
    executed_at TIMESTAMPTZ DEFAULT NOW(),
    duration_ms INTEGER,
    error_message TEXT,
    response_data JSONB,
    retry_count INTEGER DEFAULT 0,
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE SET NULL,
    CONSTRAINT fk_event FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE SET NULL
);

CREATE INDEX idx_action_results_trigger_id ON action_results(trigger_id);
CREATE INDEX idx_action_results_status ON action_results(status);
CREATE INDEX idx_action_results_executed_at ON action_results(executed_at DESC);
```

**Analytics Views**:
```sql
-- Hourly action metrics
CREATE MATERIALIZED VIEW action_metrics_hourly AS
SELECT
    DATE_TRUNC('hour', executed_at) as hour,
    action_type,
    COUNT(*) as total_executions,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as success_count,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failure_count,
    AVG(duration_ms) as avg_duration_ms
FROM action_results
GROUP BY hour, action_type;
```

#### 9. Output Channels

**Telegram Bot API**:
- Bot token configured per user or system-wide
- Chat ID specified in trigger action config
- Rate limits: 30 messages/second globally

**HTTP REST APIs**:
- User-configured webhook endpoints
- Standard HTTP methods and headers
- Authentication via headers (API keys, OAuth tokens)

**MCP Protocol Servers**:
- Agent-controlled servers exposing tools/resources
- Endpoint URLs retrieved from agent registration file (tokenURI)
- Authentication via MCP protocol mechanisms (to be determined by agent)

### Data Flow Example

Complete flow for a reputation feedback event:

```
1. Client submits NewFeedback transaction to ReputationRegistry contract on Base Sepolia
   ↓
2. RPC Node (Alchemy) detects new block with transaction
   ↓
3. Ponder Indexer (Base Sepolia) processes onNewFeedback event
   ↓
4. Event normalized and written to Event Store (PostgreSQL)
   ↓
5. PostgreSQL NOTIFY triggers on 'new_event' channel
   ↓
6. Event Processor (Rust) receives notification
   ↓
7. Query Trigger Store for triggers matching:
   - chain_id = 84532 (Base Sepolia)
   - registry = 'reputation'
   - enabled = true
   ↓
8. For each matching trigger:
   a. Evaluate conditions (e.g., score < 60 AND agent_id = 42)
   b. Update stateful triggers (EMA, rate counters)
   c. Check rate limits and circuit breaker
   ↓
9. If trigger matches:
   a. Create job for each action (Telegram, MCP)
   b. Enqueue jobs to Redis (with priority)
   ↓
10. Action Workers consume jobs from Redis:
    - Telegram Worker: Send formatted message to chat
    - MCP Worker:
      a. Fetch tokenURI from Identity Registry
      b. Parse registration file for MCP endpoint
      c. Fetch IPFS file content
      d. Verify file hash
      e. POST to agent's MCP server: agent.receiveFeedback
   ↓
11. Log action results to Result Logger (PostgreSQL)
   ↓
12. Agent receives feedback via MCP and updates internal model
```

## Tech Stack

### Core Technologies

#### Backend Services (Rust)
- **Actix-web 4.x** - High-performance async web framework
- **Tokio 1.x** - Async runtime for concurrent processing
- **SQLx 0.7.x** - Compile-time verified SQL queries with async support
- **Reqwest 0.11.x** - HTTP client for REST and MCP communication
- **Serde 1.x** - JSON serialization/deserialization
- **Validator** - Input validation
- **Thiserror** - Error handling
- **Tracing** - Structured logging
- **Redis client** - Job queue and caching

#### Blockchain Indexing (TypeScript)
- **Ponder 0.4.x** - Purpose-built blockchain indexing framework
- **Viem 2.x** - Type-safe Ethereum library
- **Node.js 20+** - Runtime environment

#### Database
- **PostgreSQL 15+** - Primary database
- **TimescaleDB 2.x** - Time-series extension for events
- **SQLx-CLI** - Migration management

#### Message Queue
- **Redis 7.x** - Job queuing and caching
- **BullMQ** (optional) - Advanced queue management

#### External Services
- **Alchemy** - Primary RPC provider
- **Infura** - Fallback RPC provider
- **Pinata/Web3.Storage** - IPFS file fetching

#### MCP Integration
- **@modelcontextprotocol/sdk** (TypeScript) - Official MCP SDK
- Rust bridge via HTTP or subprocess

### Development Tools

- **Cargo** - Rust package manager and build tool
- **pnpm** - Node.js package manager
- **Docker & Docker Compose** - Containerization and local development
- **Git** - Version control
- **GitHub Actions** - CI/CD pipelines

### Testing

- **Rust**:
  - cargo test - Unit and integration tests
  - criterion - Benchmarking
  - mockall - Mocking

- **TypeScript**:
  - Vitest - Unit testing
  - Ponder test utilities

### Observability

- **Prometheus** - Metrics collection
- **Grafana** - Metrics visualization
- **Loki** - Log aggregation
- **Tracing** - Distributed tracing (Jaeger/Tempo)

## Development Guidelines

### Project Structure

```
api.8004.dev/
├── CLAUDE.md                    # This file
├── README.md                    # User-facing documentation
├── docs/                        # Detailed documentation
│   ├── architecture/
│   ├── api/
│   ├── protocols/
│   ├── database/
│   ├── operations/
│   ├── development/
│   └── examples/
├── rust-backend/                # Rust workspace
│   ├── Cargo.toml              # Workspace manifest
│   ├── crates/
│   │   ├── api-gateway/        # REST API server
│   │   ├── event-processor/    # Trigger matching engine
│   │   ├── action-workers/     # Action execution workers
│   │   └── shared/             # Shared libraries
│   │       ├── db/             # Database utilities
│   │       ├── models/         # Common data models
│   │       ├── mcp/            # MCP protocol client
│   │       └── utils/          # Helpers
│   └── tests/
│       ├── integration/
│       └── e2e/
├── ponder-indexers/            # Blockchain indexers
│   ├── package.json
│   ├── ponder.config.ts        # Multi-chain configuration
│   ├── src/
│   │   ├── identity-registry.ts
│   │   ├── reputation-registry.ts
│   │   └── validation-registry.ts
│   ├── abis/                   # Contract ABIs
│   └── tests/
├── database/
│   ├── migrations/             # SQL migrations
│   ├── seeds/                  # Test data
│   └── schema.sql              # Full schema reference
├── scripts/
│   ├── setup-dev.sh            # Local environment setup
│   ├── run-tests.sh            # Test runner
│   └── deploy.sh               # Deployment script
├── docker/
│   ├── docker-compose.yml      # Local development stack
│   ├── docker-compose.prod.yml # Production configuration
│   ├── api-gateway.Dockerfile
│   ├── event-processor.Dockerfile
│   ├── action-workers.Dockerfile
│   └── ponder.Dockerfile
└── .github/
    └── workflows/
        ├── ci.yml              # Continuous integration
        └── deploy.yml          # Deployment pipeline
```

### Quality Standards & Testing Policy

**CRITICAL RULE: 100% Test Coverage Before Commits**

All code changes MUST have passing tests before being committed to the repository. This is a non-negotiable requirement for maintaining code quality and preventing regressions.

#### Testing Requirements

1. **Pre-Commit Verification**:
   - ALL tests must pass before creating a commit
   - Run the full test suite: `./scripts/run-tests.sh`
   - No commits with failing tests are allowed
   - No commits without tests for new functionality

2. **Test Coverage Requirements**:
   - **Database**: 100% of migrations, schema changes, and queries must be tested
   - **Backend Services (Rust)**: Minimum 80% code coverage for all crates
   - **Ponder Indexers (TypeScript)**: 100% of event handlers must be tested
   - **API Endpoints**: 100% of endpoints must have integration tests
   - **Critical Paths**: 100% coverage for trigger matching, action execution, and state management

3. **Test Types Required**:

   **Unit Tests**:
   - Test individual functions and modules in isolation
   - Mock external dependencies
   - Fast execution (<1s per test suite)
   - Example: Database query functions, condition evaluators, parsers

   **Integration Tests**:
   - Test component interactions
   - Use test database with migrations applied
   - Test API → Database, Event Processor → Queue flows
   - Example: API endpoint creates trigger in database, event triggers action

   **Database Tests**:
   - Verify migrations apply correctly
   - Test constraints, indexes, and triggers
   - Verify data integrity rules
   - Test rollback scenarios
   - Example: Foreign key cascades, unique constraints, TimescaleDB hypertable behavior

   **End-to-End Tests**:
   - Test complete workflows
   - Simulate real-world scenarios
   - Example: Blockchain event → trigger match → notification sent

4. **Test Execution Workflow**:

   ```bash
   # Before committing
   ./scripts/run-tests.sh

   # Individual test suites
   cd rust-backend && cargo test           # Rust tests
   cd ponder-indexers && pnpm test         # TypeScript tests
   cd database && ./test-migrations.sh      # Database tests
   ```

5. **Continuous Integration**:
   - GitHub Actions runs all tests on every PR
   - PRs cannot be merged if tests fail
   - Test coverage reports are generated automatically
   - Failing tests block deployment to all environments

6. **Test Documentation**:
   - Each test file must include a header comment explaining what is being tested
   - Complex test scenarios must have inline comments
   - Test naming convention: `test_<functionality>_<scenario>_<expected_outcome>`
   - Example: `test_trigger_matching_score_threshold_below_60_matches`

7. **Test Data Management**:
   - Use `database/seeds/test_data.sql` for consistent test data
   - Reset database state between test runs
   - Never use production data in tests
   - Clean up test data after test execution

#### Enforcement

- **Git Pre-Commit Hook**: Automatically runs tests before allowing commit
- **CI Pipeline**: Blocks PRs with failing tests
- **Code Review**: Reviewers must verify test coverage
- **No Exceptions**: Even "minor" changes require tests

#### Testing Philosophy

> "If it's not tested, it's broken. If it's not automatically tested, it will break."

Tests are not optional. They are:
- **Documentation**: Tests show how code should be used
- **Safety Net**: Prevent regressions when refactoring
- **Design Tool**: Writing tests first improves API design
- **Confidence**: Deploy knowing the system works

### Coding Standards

#### Rust

**General Principles**:
- Use meaningful variable and function names
- Keep functions focused and small (<100 lines)
- Prefer composition over inheritance
- Use the type system to prevent errors at compile time
- Document public APIs with rustdoc comments

**Error Handling**:
```rust
// Use thiserror for library errors
#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid trigger configuration: {0}")]
    InvalidConfig(String),

    #[error("Trigger {0} not found")]
    NotFound(String),
}

// Use anyhow for application errors
use anyhow::{Context, Result};

pub async fn process_event(event_id: &str) -> Result<()> {
    let event = fetch_event(event_id)
        .await
        .context("Failed to fetch event from database")?;

    // ... processing logic

    Ok(())
}
```

**Async Patterns**:
```rust
// Prefer async/await over manual Future manipulation
pub async fn fetch_triggers(chain_id: i32, registry: &str) -> Result<Vec<Trigger>> {
    sqlx::query_as!(
        Trigger,
        r#"
        SELECT * FROM triggers
        WHERE chain_id = $1 AND registry = $2 AND enabled = true
        "#,
        chain_id,
        registry
    )
    .fetch_all(&pool)
    .await
    .context("Failed to fetch triggers")
}

// Use tokio::spawn for concurrent tasks
let handles: Vec<_> = triggers
    .into_iter()
    .map(|trigger| {
        tokio::spawn(async move {
            evaluate_trigger(trigger).await
        })
    })
    .collect();

let results = futures::future::join_all(handles).await;
```

**Logging**:
```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(pool), fields(trigger_id = %trigger.id))]
pub async fn evaluate_trigger(
    trigger: Trigger,
    event: Event,
    pool: &PgPool
) -> Result<bool> {
    debug!("Evaluating trigger conditions");

    let matches = check_conditions(&trigger, &event)?;

    if matches {
        info!("Trigger matched, enqueueing actions");
    } else {
        debug!("Trigger did not match");
    }

    Ok(matches)
}
```

**Testing**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_score_threshold_condition() {
        let condition = Condition {
            condition_type: "score_threshold".to_string(),
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            config: None,
        };

        let event = Event {
            score: Some(45),
            ..Default::default()
        };

        assert!(evaluate_condition(&condition, &event).unwrap());
    }
}
```

#### TypeScript (Ponder)

**General Principles**:
- Enable strict mode in tsconfig.json
- Use explicit types, avoid `any`
- Prefer immutability (const, readonly)
- Use async/await consistently

**Event Handlers**:
```typescript
import { ponder } from "@/generated";

ponder.on("ReputationRegistry:NewFeedback", async ({ event, context }) => {
  const { agentId, clientAddress, score, tag1, tag2, fileUri, fileHash } = event.args;

  // Normalize tags from bytes32 to string
  const tag1Str = tag1 ? bytes32ToString(tag1) : null;
  const tag2Str = tag2 ? bytes32ToString(tag2) : null;

  // Insert into Event Store
  await context.db.insert("events", {
    id: `${context.network.chainId}-${event.block.number}-${event.logIndex}`,
    chainId: context.network.chainId,
    blockNumber: event.block.number,
    blockHash: event.block.hash,
    transactionHash: event.transaction.hash,
    logIndex: event.logIndex,
    registry: "reputation",
    eventType: "NewFeedback",
    agentId: BigInt(agentId),
    clientAddress,
    score: Number(score),
    tag1: tag1Str,
    tag2: tag2Str,
    fileUri,
    fileHash,
    timestamp: event.block.timestamp,
  });
});
```

**Configuration**:
```typescript
import { createConfig } from "@ponder/core";
import { http } from "viem";

export default createConfig({
  networks: {
    baseSepolia: {
      chainId: 84532,
      transport: http(process.env.BASE_SEPOLIA_RPC_URL),
    },
    sepolia: {
      chainId: 11155111,
      transport: http(process.env.SEPOLIA_RPC_URL),
    },
    // ... other networks
  },
  contracts: {
    ReputationRegistry: {
      network: {
        baseSepolia: {
          address: "0x...",
          startBlock: 1234567,
        },
        sepolia: {
          address: "0x...",
          startBlock: 7654321,
        },
      },
      abi: "./abis/ReputationRegistry.json",
    },
    // ... other contracts
  },
});
```

### Database Conventions

#### Migration Strategy

- Use SQLx migrations for Rust projects: `sqlx migrate add <name>`
- Migrations must be reversible (provide both `up` and `down`)
- Never modify existing migrations after deployment
- Include descriptive comments in migration files
- Test migrations on local database before committing

**Example Migration**:
```sql
-- migrations/20250123_create_triggers_table.up.sql

-- Create triggers table with full audit trail
CREATE TABLE triggers (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    chain_id INTEGER NOT NULL,
    registry TEXT NOT NULL CHECK (registry IN ('identity', 'reputation', 'validation')),
    enabled BOOLEAN DEFAULT true,
    is_stateful BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create index for common query patterns
CREATE INDEX idx_triggers_user_id ON triggers(user_id);
CREATE INDEX idx_triggers_chain_registry_enabled
    ON triggers(chain_id, registry, enabled)
    WHERE enabled = true;

-- Create updated_at trigger
CREATE TRIGGER update_triggers_updated_at
    BEFORE UPDATE ON triggers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- migrations/20250123_create_triggers_table.down.sql
DROP TRIGGER IF EXISTS update_triggers_updated_at ON triggers;
DROP INDEX IF EXISTS idx_triggers_chain_registry_enabled;
DROP INDEX IF EXISTS idx_triggers_user_id;
DROP TABLE IF EXISTS triggers;
```

#### Indexing Patterns

- Index foreign keys for JOIN performance
- Use partial indexes for filtered queries (e.g., `WHERE enabled = true`)
- Create covering indexes for frequently accessed columns
- Use GIN indexes for JSONB columns with frequent queries
- Monitor query performance with `EXPLAIN ANALYZE`

#### Partitioning Strategy

- Use TimescaleDB hypertables for time-series data (events table)
- Partition by time with appropriate chunk intervals (7 days for events)
- Consider declarative partitioning for large lookup tables
- Implement retention policies for old data

### API Design

#### REST Conventions

- Use standard HTTP methods: GET (read), POST (create), PUT (replace), PATCH (update), DELETE (remove)
- Use plural nouns for resource endpoints: `/api/v1/triggers`, not `/api/v1/trigger`
- Use HTTP status codes correctly:
  - 200 OK - Successful GET, PUT, PATCH
  - 201 Created - Successful POST
  - 204 No Content - Successful DELETE
  - 400 Bad Request - Invalid input
  - 401 Unauthorized - Missing or invalid authentication
  - 403 Forbidden - Authenticated but not authorized
  - 404 Not Found - Resource doesn't exist
  - 409 Conflict - Resource conflict (e.g., duplicate)
  - 500 Internal Server Error - Server error

#### Authentication/Authorization

- Use JWT tokens for authentication
- Include user_id in JWT claims
- Validate JWT on every protected endpoint
- Enforce resource ownership (users can only access their own triggers)
- Use middleware for auth checks

#### Versioning

- Include version in URL path: `/api/v1/...`
- Never break backward compatibility within a version
- Deprecate old versions with clear migration path and timeline

#### Request/Response Format

**Successful Response**:
```json
{
  "data": {
    "id": "trigger_123",
    "name": "Low Score Alert",
    "enabled": true
  }
}
```

**Error Response**:
```json
{
  "error": {
    "code": "INVALID_TRIGGER_CONFIG",
    "message": "Score threshold must be between 0 and 100",
    "details": {
      "field": "conditions[0].value",
      "value": "150"
    }
  }
}
```

**Paginated Response**:
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total_pages": 5,
    "total_items": 93
  }
}
```

## Testing Strategy

### Unit Tests

- Test individual functions and modules in isolation
- Mock external dependencies (database, HTTP clients)
- Aim for >80% code coverage
- Run fast (<1s per test suite)

**Rust Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn test_parse_score_threshold() {
        let condition = Condition {
            condition_type: "score_threshold".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            ..Default::default()
        };

        let threshold = parse_threshold(&condition).unwrap();
        assert_eq!(threshold, 60);
    }
}
```

### Integration Tests

- Test component interactions (API → Database, Event Processor → Queue)
- Use test database with migrations applied
- Reset state between tests
- Test error conditions and edge cases

**Rust Example**:
```rust
#[sqlx::test]
async fn test_create_trigger(pool: PgPool) -> sqlx::Result<()> {
    let trigger = CreateTriggerRequest {
        name: "Test Trigger".to_string(),
        chain_id: 84532,
        registry: "reputation".to_string(),
        conditions: vec![...],
        actions: vec![...],
    };

    let created = create_trigger(&pool, "user_123", trigger).await?;

    assert!(created.id.len() > 0);
    assert_eq!(created.name, "Test Trigger");

    Ok(())
}
```

### End-to-End Tests

- Test complete workflows (blockchain event → notification sent)
- Use testnet or local blockchain (Anvil/Hardhat)
- Verify final outcomes in external systems
- Run as part of pre-deployment checks

### Load Testing

- Use k6 or Artillery for HTTP load testing
- Simulate high event throughput (1000+ events/sec)
- Test queue backpressure handling
- Identify bottlenecks and optimization opportunities

## Deployment

### Environment Configuration

Separate configurations for development, staging, and production:

**Development**:
- Local PostgreSQL and Redis (Docker Compose)
- Testnet RPC endpoints
- Verbose logging
- Hot reload enabled

**Staging**:
- Managed PostgreSQL (AWS RDS, Render)
- Managed Redis (AWS ElastiCache, Upstash)
- Production-like configuration
- Integration with testnet contracts

**Production**:
- High-availability database with replicas
- Redis cluster
- Multiple RPC providers with failover
- Structured logging to Loki/CloudWatch
- Metrics exported to Prometheus

### Database Setup

1. Install PostgreSQL 15+ with TimescaleDB extension
2. Create database: `createdb erc8004_backend`
3. Enable TimescaleDB: `CREATE EXTENSION IF NOT EXISTS timescaledb;`
4. Run migrations: `sqlx migrate run`
5. (Optional) Seed test data: `psql < database/seeds/test_triggers.sql`

### Monitoring & Observability

**Metrics** (Prometheus):
- Request rate, latency, error rate per endpoint
- Event processing rate per chain
- Action execution success/failure rates
- Queue depth and processing lag
- Database connection pool utilization

**Logging** (Structured JSON):
- All HTTP requests/responses
- Trigger evaluations (match/no match)
- Action executions with outcomes
- Errors with full context

**Alerting**:
- Error rate >5% for any component
- Action execution latency >30s (p95)
- Queue depth >10,000 jobs
- Database connection pool exhausted
- RPC provider failures

**Dashboards** (Grafana):
- System overview (requests, events, actions)
- Per-chain event rates
- Per-trigger execution stats
- Action worker performance
- Database query performance

## MCP Integration

### Protocol Implementation

The Model Context Protocol (MCP) enables standardized communication between AI agents and external systems. In this project, MCP serves as the bridge for pushing on-chain feedback to off-chain agents.

**MCP SDK**: We use the official TypeScript SDK from `@modelcontextprotocol/sdk`.

**Architecture**:
```
Rust Action Worker
  ↓ HTTP/IPC
TypeScript MCP Bridge Service
  ↓ MCP Protocol
Agent's MCP Server
```

**MCP Bridge Service** (TypeScript):
```typescript
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

export async function sendFeedbackToAgent(
  mcpEndpoint: string,
  toolName: string,
  payload: FeedbackPayload
): Promise<void> {
  const transport = new StdioClientTransport({
    command: mcpEndpoint.command,
    args: mcpEndpoint.args,
  });

  const client = new Client({
    name: "erc8004-backend",
    version: "1.0.0",
  }, {
    capabilities: {},
  });

  await client.connect(transport);

  // Call the agent's feedback tool
  const result = await client.callTool({
    name: toolName,
    arguments: payload,
  });

  await client.close();

  return result;
}
```

### Agent Communication Patterns

**Push Notifications** (most common):
1. Event occurs on-chain (NewFeedback, ValidationResponse)
2. Trigger matches and enqueues MCP action
3. MCP worker resolves agent endpoint from tokenURI
4. Worker fetches and verifies IPFS file content
5. Worker calls agent's MCP tool with structured payload
6. Agent processes feedback and updates internal state

**Resource Updates** (future consideration):
- Agents could expose MCP resources that the backend updates
- Example: Update a "reputation_history" resource with new feedback
- Requires bidirectional MCP connection or webhook pattern

### Endpoint Discovery

Agent registration files (at tokenURI) contain MCP endpoint information:

```json
{
  "name": "Trading Agent Alpha",
  "version": "1.0.0",
  "capabilities": {
    "mcp": {
      "endpoint": "https://agent.example.com/mcp",
      "tools": [
        {
          "name": "agent.receiveFeedback",
          "description": "Receive reputation feedback from ERC-8004 backend",
          "inputSchema": {
            "type": "object",
            "properties": {
              "score": { "type": "integer" },
              "clientAddress": { "type": "string" },
              "fileContent": { "type": "object" }
            }
          }
        }
      ],
      "authentication": {
        "type": "bearer",
        "tokenHeader": "X-Agent-Token"
      }
    }
  }
}
```

**Endpoint Resolution Process**:
1. Fetch tokenURI from IdentityRegistry contract
2. Parse JSON to extract MCP endpoint and tools
3. Cache endpoint configuration (invalidate on MetadataSet events)
4. Use authentication config for MCP connection

## Security Considerations

### Authentication & Authorization

- **JWT tokens** with short expiration (1 hour) and refresh tokens
- **API rate limiting**: 100 requests/minute per user
- **Trigger ownership**: Users can only manage their own triggers
- **Action validation**: Verify webhook URLs and Telegram chat IDs belong to user

### Input Validation

- Validate all API inputs against JSON schemas
- Sanitize user-provided templates to prevent injection
- Limit trigger complexity (max 10 conditions, max 5 actions)
- Validate blockchain addresses (checksummed format)

### Rate Limiting

- Per-user API rate limits (100 req/min)
- Per-trigger execution limits (configurable, default: 100/hour)
- Per-action-type limits (e.g., max 1000 Telegram messages/day)
- Circuit breaker for failing triggers (auto-disable after 10 consecutive failures)

### Secrets Management

- Store sensitive data in environment variables or secret manager (AWS Secrets Manager, HashiCorp Vault)
- Never log API keys, tokens, or passwords
- Rotate credentials regularly
- Use encrypted connections (TLS) for all external communication

### Database Security

- Use connection pooling with max connections limit
- Parameterized queries only (SQLx enforces this)
- Row-level security policies for multi-tenant data
- Regular backups with encryption at rest

### MCP Security

- Validate agent MCP endpoints (HTTPS required)
- Implement timeout for MCP calls (30s max)
- Verify file hashes before sending to agents
- Rate limit MCP calls per agent (prevent abuse)

## Development Roadmap

### Phase 1: Foundation (Weeks 1-3)

**Deliverables**:
- ✅ CLAUDE.md and documentation structure
- Database schema design and migrations
- Docker Compose setup for local development
- Rust workspace structure with shared libraries
- Basic API Gateway with health check endpoint

**Subagents**:
- database-administrator (schema design)
- rust-engineer (workspace setup)
- devops-engineer (Docker configuration)

## Development Workflow

### Local Testing Scripts

The project includes comprehensive local testing scripts that replicate GitHub Actions workflows:

**Daily Development** (`./scripts/local-ci.sh`):
- Database tests (schema, TimescaleDB, integrity, notifications, performance)
- Rust tests (formatting, Clippy, build, unit tests)
- TypeScript tests (type-check, linting, tests)
- Runtime: 2-5 minutes

**Pre-PR Quality Checks** (`./scripts/local-lint.sh`):
- SQL linting (style, trailing whitespace)
- Rust linting (formatting, Clippy, unsafe code, TODOs)
- TypeScript linting (formatting, ESLint, type checking)
- Documentation checks (required files, broken links)
- Docker Compose validation
- Shell script linting (ShellCheck)
- Runtime: 3-5 minutes

**Security Audit** (`./scripts/local-security.sh`):
- Dependency vulnerability scanning (cargo-audit, npm audit)
- Docker image security (Trivy)
- Secrets detection (Gitleaks)
- Dockerfile linting (hadolint)
- Configuration security checks
- Runtime: 5-10 minutes

**Complete CI Replication** (`./scripts/local-all.sh`):
- Runs all three scripts in sequence
- Comprehensive summary with timing
- Use `--yes` or `-y` to skip confirmation
- Runtime: 10-15 minutes

**Usage**:
```bash
# Daily workflow validation
./scripts/local-ci.sh

# Before creating PR
./scripts/local-lint.sh

# Weekly/monthly security check
./scripts/local-security.sh

# Complete validation before pushing to main
./scripts/local-all.sh
```

### Code Quality Tools

**Required**:
- Rust: cargo, rustfmt, clippy
- Node.js: node, pnpm, typescript, eslint
- Database: psql, docker

**Optional** (for complete coverage):
- ShellCheck (shell script linting)
- cargo-audit (Rust dependency auditing)
- Trivy (container security scanning)
- Gitleaks (secrets detection)
- hadolint (Dockerfile linting)

Install on macOS:
```bash
brew install shellcheck trivy gitleaks hadolint
cargo install cargo-audit
```

Install on Ubuntu/Debian:
```bash
apt install shellcheck
# (trivy, gitleaks, hadolint: see official installation docs)
cargo install cargo-audit
```

### Phase 2: Event Ingestion (Weeks 4-6)

**Deliverables**:
- Ponder indexers for all three registries
- Event Store with TimescaleDB
- Checkpoint management and reorg handling
- PostgreSQL NOTIFY/LISTEN implementation

**Subagents**:
- typescript-pro (Ponder indexers)
- database-administrator (Event Store optimization)

### Phase 3: Core Backend (Weeks 7-10)

**Status**: Week 7 Complete (✅ API Gateway CRUD - 100%)

**Deliverables**:
- ✅ API Gateway with full CRUD for triggers (Week 7 - 100%)
- ✅ JWT authentication and user management (Week 7 - 100%)
- ⏳ Event Processor with trigger matching
- ⏳ Redis job queueing
- ⏳ Basic Telegram worker (simple triggers only)

**Subagents**:
- ✅ backend-architect (API design and implementation - Week 7)
- ✅ rust-engineer (Code review and optimization - Week 7)
- ⏳ backend-developer (Event Processor and workers)

### Phase 4: Advanced Triggers & Actions (Weeks 11-13)

**Deliverables**:
- Stateful trigger support (EMA, counters, rate limiting)
- REST/HTTP action worker
- Circuit breaker implementation
- Result Logger with analytics views

**Subagents**:
- rust-engineer (stateful triggers)
- backend-developer (action workers)

### Phase 5: MCP Integration (Weeks 14-16)

**Deliverables**:
- TypeScript MCP bridge service
- MCP worker with endpoint discovery
- IPFS file fetching and verification
- OASF schema validation

**Subagents**:
- typescript-pro (MCP bridge)
- mcp-developer (protocol implementation)
- rust-engineer (Rust-TypeScript bridge)

### Phase 6: Testing & Observability (Weeks 17-19)

**Deliverables**:
- Comprehensive test suite (unit, integration, e2e)
- Prometheus metrics implementation
- Grafana dashboards
- Structured logging with tracing
- Load testing and performance optimization

**Subagents**:
- debugger (test implementation)
- performance-engineer (optimization)
- devops-engineer (observability stack)

### Phase 7: Production Deployment (Weeks 20-22)

**Deliverables**:
- CI/CD pipelines (GitHub Actions)
- Production environment configuration
- Database backups and disaster recovery
- Security audit and hardening
- API documentation (OpenAPI/Swagger)
- User guides and examples

**Subagents**:
- deployment-engineer (CI/CD)
- security-engineer (security audit)
- api-documenter (API docs)
- documentation-engineer (user guides)

### Phase 8: AI Integration (Future)

**Deliverables**:
- Natural language trigger creation
- Event content interpretation and classification
- Trend prediction and anomaly detection
- Automated trigger optimization

**Subagents**:
- ai-engineer (AI integration)

## Additional Resources

### ERC-8004 Standard
- **EIP Specification**: https://eips.ethereum.org/EIPS/eip-8004
- **Contracts Repository**: https://github.com/erc-8004/erc-8004-contracts
- **Test Deployments**: See contracts repo for current testnet addresses

### OASF (Open Agentic Schema Framework)
- **Repository**: https://github.com/agntcy/oasf
- **Schema Validator**: https://oasf.agntcy.io (browser and validation server)
- **Latest Version**: v0.8.0

### MCP (Model Context Protocol)
- **Specification**: https://github.com/modelcontextprotocol/specification
- **TypeScript SDK**: https://github.com/modelcontextprotocol/typescript-sdk
- **Documentation**: https://modelcontextprotocol.io/docs

### Development Tools
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Tutorial**: https://tokio.rs/tokio/tutorial
- **Actix-web Guide**: https://actix.rs/docs/
- **Ponder Documentation**: https://ponder.sh/docs
- **TimescaleDB Docs**: https://docs.timescale.com/

---

**Last Updated**: November 24, 2024
**Version**: 1.0.0
**Maintainers**: Development Team
