# api.8004.dev - ERC-8004 Backend Infrastructure

## Quick Reference

### Common Commands

```bash
# Development setup
docker-compose up -d                  # Start local services (PostgreSQL, Redis)
sqlx migrate run                      # Apply database migrations
cargo run --bin api-gateway           # Start API server

# Testing
./scripts/local-ci.sh                 # Daily workflow validation (2-5 min)
./scripts/local-lint.sh               # Pre-PR quality checks (3-5 min)
./scripts/local-security.sh           # Security audit (5-10 min)
./scripts/local-all.sh                # Complete validation (10-15 min)
cargo test                            # Rust unit/integration tests
cd ponder-indexers && pnpm test       # TypeScript tests

# Database operations
sqlx migrate add <name>               # Create new migration
psql erc8004_backend                  # Connect to database
psql < database/seeds/test_data.sql   # Load test data
```

### Key Endpoints (API Gateway)

```
# User Authentication (JWT)
POST   /api/v1/auth/register          # Create user account
POST   /api/v1/auth/login             # Get JWT token

# API Key Management (Layer 1)
POST   /api/v1/api-keys               # Create API key
GET    /api/v1/api-keys               # List organization's keys
GET    /api/v1/api-keys/:id           # Get key details (masked)
DELETE /api/v1/api-keys/:id           # Revoke key
POST   /api/v1/api-keys/:id/rotate    # Rotate key

# Wallet Authentication (Layer 2)
POST   /api/v1/auth/wallet/challenge  # Request signing challenge
POST   /api/v1/auth/wallet/verify     # Submit signature, get JWT

# Agent Linking (Layer 2)
POST   /api/v1/agents/link            # Link agent to organization
GET    /api/v1/agents/linked          # List linked agents
DELETE /api/v1/agents/:agent_id/link  # Unlink agent

# Triggers (PUSH Layer) - Organization-scoped
GET    /api/v1/triggers               # List organization triggers (paginated)
POST   /api/v1/triggers               # Create new trigger
GET    /api/v1/triggers/{id}          # Get trigger details
PUT    /api/v1/triggers/{id}          # Update trigger
DELETE /api/v1/triggers/{id}          # Delete trigger

# Organizations (Phase 3.5)
POST   /api/v1/organizations          # Create organization
GET    /api/v1/organizations          # List user's organizations
GET    /api/v1/organizations/:id      # Get organization details
PUT    /api/v1/organizations/:id      # Update organization
DELETE /api/v1/organizations/:id      # Delete organization

# Organization Members
GET    /api/v1/organizations/:id/members      # List members
POST   /api/v1/organizations/:id/members      # Add member
PUT    /api/v1/organizations/:id/members/:uid # Update member role
DELETE /api/v1/organizations/:id/members/:uid # Remove member

# Billing (Pull Layer - Phase 3.5, planned)
GET    /api/v1/billing/credits        # Get credit balance
POST   /api/v1/billing/credits/purchase  # Purchase credits (Stripe)

# A2A Protocol (Pull Layer - Phase 5)
POST   /api/v1/a2a/rpc                # JSON-RPC 2.0 endpoint
GET    /api/v1/a2a/tasks/:id/stream   # SSE progress updates

# Discovery
GET    /.well-known/agent.json        # Agent Card (public)
GET    /api/v1/health                 # System health status
```

Full API documentation: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`

## Project Overview

This project provides a real-time backend infrastructure for monitoring, interpreting, and reacting to events from the ERC-8004 standard's three on-chain registries: Identity, Reputation, and Validation. It enables programmable triggers that execute automated actions based on blockchain events, creating a bridge between on-chain agent activity and off-chain systems.

### Purpose

The ERC-8004 standard defines the foundation for on-chain agent economy:
- **Who** an agent is (Identity Registry)
- **How** an agent is evaluated (Reputation Registry)
- **How** an agent's work is validated (Validation Registry)

This backend transforms these raw blockchain signals into intelligent actions: notifications, API calls, and updates to agent MCP (Model Context Protocol) servers, enabling agents to learn and adapt based on their on-chain reputation.

### Key Capabilities

**PUSH Layer** (event-driven notifications):
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

**PULL Layer** (agent-initiated queries):
- **A2A Protocol** (Google Agent-to-Agent) for async queries with task lifecycle
- **MCP Query Tools** (4 tiers):
  - Tier 0: Raw queries (feedbacks, validations, agent profile)
  - Tier 1: Aggregated queries (reputation summary, trends)
  - Tier 2: Analysis queries (client analysis, baseline comparison)
  - Tier 3: AI-powered queries (reputation reports, dispute analysis)
- **Payment System**:
  - Stripe integration (fiat)
  - x402 protocol (crypto)
  - Credits system (prepaid)
- **Multi-tenant Account Model** with organizations and role-based access

**Authentication System** (3-layer model):
- **Layer 0: Anonymous** - No authentication required
  - x402 payment only (crypto micropayments)
  - IP-based rate limiting (10 calls/hour)
  - Tier 0-1 queries only
- **Layer 1: API Key** - Account-based authentication
  - Format: `sk_live_xxx` (production) / `sk_test_xxx` (testing)
  - Argon2id hashing with OWASP-recommended parameters (64MiB memory, 3 iterations)
  - **Security hardening**:
    - Timing attack mitigation via pre-computed dummy hash for constant-time verification
    - Authentication rate limiting: 20 attempts/min per IP, 1000/min global
    - Dual audit logging: `api_key_audit_log` (org-scoped) + `auth_failures` (pre-org failures)
  - All payment methods (Stripe, x402, Credits)
  - Per-plan rate limits (Starter: 100/hr, Pro: 500/hr, Enterprise: 2000/hr)
  - Full access to Tier 0-3 queries
- **Layer 2: Wallet Signature** - On-chain agent authentication
  - EIP-191 signature verification
  - Agent → Account linking via challenge-response
  - On-chain ownership verification (IdentityRegistry.ownerOf)
  - Nonce management for replay attack prevention
  - Inherits account permissions and rate limits

See [Authentication Documentation](docs/auth/AUTHENTICATION.md) for complete details.

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
- **3-layer authentication** (Anonymous, API Key, Wallet Signature)
- JWT authentication middleware (jsonwebtoken crate)
- **DualAuth middleware** supporting both JWT and API Key authentication
- API Key authentication middleware (`sk_live_xxx` format) with security hardening:
  - **Timing attack mitigation**: Pre-computed Argon2id hash for dummy_verify()
  - **Authentication rate limiting**: Governor crate (20 auth/min per IP, 1000/min global)
  - **Comprehensive audit logging**: 2-tier system (api_key_audit_log + auth_failures)
- EIP-191 wallet signature verification (alloy crate)
- Argon2id hashing for passwords and API keys (OWASP parameters: 64MiB, 3 iterations)
- User ownership validation on all trigger operations
- CORS whitelist with environment configuration
- Input validation with validator crate
- Redis-based rate limiting (per-tier, per-account, per-IP)

**Architecture**:
- 3-layer design: Handlers → Repositories → Database
- Repository pattern for clean database access
- DTO pattern for request/response serialization
- Compile-time SQL verification with SQLx
- Pagination support (limit/offset parameters)

**Documentation**: See `rust-backend/crates/api-gateway/API_DOCUMENTATION.md` for complete API reference with examples.

#### 2. RPC Nodes (External Services)

**Responsibility**: Blockchain data access via JSON-RPC protocol.

**Providers**: Alchemy (primary), Infura (fallback), QuickNode (additional fallback)

**Features**:
- Connection pooling for throughput
- Automatic retry with exponential backoff
- Load balancing across multiple providers
- Response caching for frequently accessed data
- Rate limit handling

#### 3. Ponder Indexer Layer (TypeScript)

**Responsibility**: Real-time blockchain event monitoring, normalization, and persistence.

**Technology Stack**: Ponder (blockchain indexing framework), Viem (Ethereum interactions), Node.js runtime

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

**Supported Chains** (Initial): Ethereum Sepolia, Base Sepolia, Linea Sepolia, Polygon Amoy

#### 4. Trigger Store (PostgreSQL)

**Responsibility**: Persistent storage of user-defined trigger configurations with multi-tenant organization support.

**Key Tables**:
- `organizations` - Multi-tenant organizations (name, slug, owner_id, plan, is_personal)
- `organization_members` - User membership in organizations (role: admin, member, viewer)
- `triggers` - Trigger definitions (organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
- `trigger_conditions` - Matching conditions (condition_type, field, operator, value, config JSONB)
- `trigger_actions` - Actions to execute (action_type, priority, config JSONB)
- `trigger_state` - State for stateful triggers (state_data JSONB)

See `database/migrations/` for complete schema definitions.

**Key Indexes**:
- `idx_triggers_organization_id` - Organization trigger lookups
- `idx_triggers_org_chain_registry_enabled` - Fast trigger matching (partial index on enabled=true)
- `idx_org_members_org`, `idx_org_members_user` - Organization membership lookups
- Foreign key indexes for joins

**Foreign Key Constraints**:
- `organizations.owner_id` → `users.id` (ON DELETE RESTRICT)
- `triggers.organization_id` → `organizations.id` (ON DELETE CASCADE)

#### 5. Event Store (PostgreSQL + TimescaleDB)

**Responsibility**: Immutable log of all blockchain events for audit, analytics, and recovery.

**Key Tables**:
- `events` - Main event table (TimescaleDB hypertable, partitioned by created_at)
  - Common fields: chain_id, block_number, registry, event_type, agent_id, timestamp
  - Registry-specific fields: owner, token_uri, score, tags, validator_address, etc.
- `checkpoints` - Last processed block per chain

**Key Features**:
- PostgreSQL NOTIFY trigger on INSERT (notifies Event Processor)
- TimescaleDB continuous aggregates for analytics (events_hourly)
- Retention policies for old data (configurable)
- Optimized indexes for common query patterns

See `database/migrations/` for complete schema and indexes.

#### 6. Event Processor (Rust/Tokio)

**Responsibility**: Core trigger matching engine that evaluates events against user-defined conditions.

**Technology Stack**: Tokio (async runtime), SQLx (database access), Redis client (job queueing)

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

**Rate Limiting & Circuit Breaking**:
- Per-trigger execution limits (default: max 10 executions/hour)
- Redis-based sliding window counters
- Auto-disables triggers with >80% failure rate
- Auto-recovery after configurable timeout (default: 1 hour)

#### 7. Action Workers (Rust/Tokio)

**Responsibility**: Execute actions in response to matched triggers.

**Technology Stack**: Tokio (async runtime), Reqwest (HTTP client), Teloxide (Telegram bot SDK), TypeScript MCP SDK (bridged from Rust)

**Worker Types**:

##### Telegram Worker
Sends formatted notifications via Telegram Bot API.

**Features**: Message templates with variable substitution, Markdown/HTML formatting, automatic retry on rate limits, message chunking

##### REST/HTTP Worker
Executes HTTP requests to external APIs.

**Features**: Support for GET/POST/PUT/DELETE/PATCH methods, custom headers, request body templating, response validation, timeout configuration (default: 30s)

##### MCP Server Worker
Pushes updates to agent MCP servers using the Model Context Protocol.

**Features**: Automatic endpoint resolution from registration file (tokenURI), file hash verification, IPFS content fetching and validation, OASF schema validation, MCP authentication handling

**Implementation**: TypeScript MCP SDK (@modelcontextprotocol/sdk), bridged from Rust via HTTP subprocess or embedded runtime, caches agent endpoint configurations

**Common Worker Features**:
- Exponential backoff retry (3 attempts: 1s, 2s, 4s)
- Dead Letter Queue (DLQ) for permanent failures
- Result logging to PostgreSQL
- Prometheus metrics for observability

#### 8. Result Logger (PostgreSQL)

**Responsibility**: Audit trail of all action executions.

**Key Tables**:
- `action_results` - Execution logs (job_id, trigger_id, event_id, action_type, status, duration_ms, error_message, retry_count)
- `action_metrics_hourly` - Materialized view for analytics (aggregated by hour, action_type with success/failure counts)

See `database/migrations/` for complete schema.

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
- Authentication via MCP protocol mechanisms (determined by agent)

### Data Flow Example

Complete flow for a reputation feedback event:

```
1. Client submits NewFeedback transaction to ReputationRegistry contract on Base Sepolia
2. RPC Node (Alchemy) detects new block with transaction
3. Ponder Indexer (Base Sepolia) processes onNewFeedback event
4. Event normalized and written to Event Store (PostgreSQL)
5. PostgreSQL NOTIFY triggers on 'new_event' channel
6. Event Processor (Rust) receives notification
7. Query Trigger Store for triggers matching: chain_id=84532, registry='reputation', enabled=true
8. For each matching trigger:
   - Evaluate conditions (e.g., score < 60 AND agent_id = 42)
   - Update stateful triggers (EMA, rate counters)
   - Check rate limits and circuit breaker
9. If trigger matches:
   - Create job for each action (Telegram, MCP)
   - Enqueue jobs to Redis (with priority)
10. Action Workers consume jobs from Redis:
    - Telegram Worker: Send formatted message to chat
    - MCP Worker: Fetch tokenURI → Parse MCP endpoint → Fetch IPFS file → Verify hash → POST to agent's MCP server
11. Log action results to Result Logger (PostgreSQL)
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

#### Message Queue & External Services
- **Redis 7.x** - Job queuing and caching
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

### Testing & Observability

- **Testing**: cargo test, criterion (benchmarking), mockall (mocking), Vitest (TypeScript)
- **Observability**: Prometheus (metrics), Grafana (dashboards), Loki (logs), Tracing (distributed tracing)

## Development Guidelines

### Project Structure

```
api.8004.dev/
├── CLAUDE.md                    # This file
├── README.md                    # User-facing documentation
├── docs/                        # Detailed documentation
├── rust-backend/                # Rust workspace
│   ├── Cargo.toml              # Workspace manifest
│   ├── crates/
│   │   ├── api-gateway/        # REST API server
│   │   ├── event-processor/    # Trigger matching engine
│   │   ├── action-workers/     # Action execution workers
│   │   └── shared/             # Shared libraries (db, models, mcp, utils)
│   └── tests/                  # Integration and e2e tests
├── ponder-indexers/            # Blockchain indexers
│   ├── ponder.config.ts        # Multi-chain configuration
│   ├── src/                    # Event handlers per registry
│   ├── abis/                   # Contract ABIs
│   └── tests/
├── database/
│   ├── migrations/             # SQL migrations
│   ├── seeds/                  # Test data
│   └── schema.sql              # Full schema reference
├── scripts/                    # Setup, testing, deployment scripts
├── docker/                     # Dockerfiles and docker-compose configs
└── .github/workflows/          # CI/CD pipelines
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
   - Test component interactions (API → Database, Event Processor → Queue)
   - Use test database with migrations applied
   - Example: API endpoint creates trigger in database, event triggers action

   **Database Tests**:
   - Verify migrations apply correctly
   - Test constraints, indexes, and triggers
   - Example: Foreign key cascades, unique constraints, TimescaleDB hypertable behavior

   **End-to-End Tests**:
   - Test complete workflows
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
        r#"SELECT * FROM triggers WHERE chain_id = $1 AND registry = $2 AND enabled = true"#,
        chain_id, registry
    )
    .fetch_all(&pool)
    .await
    .context("Failed to fetch triggers")
}

// Use tokio::spawn for concurrent tasks
let handles: Vec<_> = triggers.into_iter()
    .map(|trigger| tokio::spawn(async move { evaluate_trigger(trigger).await }))
    .collect();
let results = futures::future::join_all(handles).await;
```

**Logging**:
```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(pool), fields(trigger_id = %trigger.id))]
pub async fn evaluate_trigger(trigger: Trigger, event: Event, pool: &PgPool) -> Result<bool> {
    debug!("Evaluating trigger conditions");
    let matches = check_conditions(&trigger, &event)?;
    if matches { info!("Trigger matched, enqueueing actions"); }
    Ok(matches)
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

### Database Conventions

#### Migration Strategy

- Use SQLx migrations for Rust projects: `sqlx migrate add <name>`
- Migrations must be reversible (provide both `up` and `down`)
- Never modify existing migrations after deployment
- Include descriptive comments in migration files
- Test migrations on local database before committing

**Example Migration** (multi-tenant pattern):
```sql
-- migrations/20250125_create_organizations_table.up.sql
CREATE TABLE organizations (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    owner_id TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free',
    is_personal BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_owner FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_organizations_owner ON organizations(owner_id);
CREATE INDEX idx_organizations_slug ON organizations(slug);

-- migrations/20250125_add_organization_to_triggers.up.sql
ALTER TABLE triggers ADD COLUMN organization_id TEXT NOT NULL;
ALTER TABLE triggers ADD CONSTRAINT fk_organization
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;

CREATE INDEX idx_triggers_organization_id ON triggers(organization_id);
CREATE INDEX idx_triggers_org_chain_registry_enabled
    ON triggers(organization_id, chain_id, registry, enabled) WHERE enabled = true;
```

#### Indexing & Partitioning Patterns

- Index foreign keys for JOIN performance
- Use partial indexes for filtered queries (e.g., `WHERE enabled = true`)
- Create covering indexes for frequently accessed columns
- Use GIN indexes for JSONB columns with frequent queries
- Use TimescaleDB hypertables for time-series data (events table)
- Partition by time with appropriate chunk intervals (7 days for events)
- Monitor query performance with `EXPLAIN ANALYZE`

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
    "details": { "field": "conditions[0].value", "value": "150" }
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
cargo install cargo-audit
# (trivy, gitleaks, hadolint: see official installation docs)
```

## Deployment

### Environment Configuration

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
Rust Action Worker → TypeScript MCP Bridge Service → Agent's MCP Server
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
  }, { capabilities: {} });

  await client.connect(transport);
  const result = await client.callTool({ name: toolName, arguments: payload });
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
      "tools": [{
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
      }],
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

### Phase 2: Event Ingestion (Weeks 4-6)

**Deliverables**:
- Ponder indexers for all three registries
- Event Store with TimescaleDB
- Checkpoint management and reorg handling
- PostgreSQL NOTIFY/LISTEN implementation

### Phase 3: Core Backend (Weeks 7-10)

**Status**: ✅ COMPLETE (100%)

**Deliverables**:
- ✅ API Gateway with full CRUD for triggers (Week 7 - 100%)
- ✅ JWT authentication and user management (Week 7 - 100%)
- ✅ Event Processor with trigger matching (Week 8 - 100%)
- ✅ Telegram Worker with security hardening (Week 9 - 100%)
- ✅ Integration Testing (Week 10 - 100%)
  - 206 total tests across workspace
  - 80 new api-gateway tests (middleware, models, validation)
  - Comprehensive coverage for DTOs and validators

### Phase 3.5: Payment Foundation (Weeks 11-12)

**Goal**: Establish multi-tenant account model and payment infrastructure (critical path for Pull Layer).

**Week 11: Organizations + API Key Auth (Layer 1)** ✅ COMPLETE
- ✅ Database migrations: `organizations`, `organization_members`, `api_keys`, `api_key_audit_log`, `auth_failures`
- ✅ Organization CRUD endpoints with role-based access
- ✅ API Key management endpoints (create, list, get, revoke, rotate)
- ✅ DualAuth middleware (JWT + API Key authentication)
- ✅ **Security hardening**:
  - Timing attack mitigation (pre-computed dummy hash)
  - Authentication rate limiting (Governor crate: 20/min IP, 1000/min global)
  - Dual audit logging system
- 170 tests passing (8 new for rate limiter)

**Week 12: Credits System + Wallet Auth (Layer 2)**
- Database migrations: `credits`, `credit_transactions`, `subscriptions`, `agent_links`
- Credit balance and transaction endpoints
- Stripe webhook integration
- Wallet signature authentication (EIP-191)

### Phase 4: Advanced Triggers & Actions (Weeks 13-15) - SHIFTED +2

**Week 13: Stateful Triggers + Rate Limiting**
- Stateful trigger support (EMA, counters)
- API rate limiting infrastructure
- Per-trigger execution limits

**Week 14: REST Worker + Discovery**
- REST/HTTP action worker
- Discovery endpoint (`/.well-known/agent.json`)
- Agent Card generation

**Week 15: Circuit Breaker + Payment Nonces**
- Circuit breaker implementation
- Payment nonces table for idempotency
- Result Logger with analytics views

### Phase 5: MCP + A2A Integration (Weeks 16-18) - EXTENDED

**Week 16: MCP Bridge + A2A Protocol**
- TypeScript MCP bridge service
- A2A Protocol JSON-RPC endpoint (`/api/v1/a2a/rpc`)
- Task lifecycle management (submitted → working → completed)
- SSE streaming for progress updates

**Week 17: MCP Worker + Query Tools Tier 0-2**
- MCP worker with endpoint discovery
- IPFS file fetching and verification
- Tier 0 tools: `getMyFeedbacks`, `getValidationHistory`, `getAgentProfile`
- Tier 1 tools: `getReputationSummary`, `getReputationTrend`
- Tier 2 tools: `getClientAnalysis`, `compareToBaseline`

**Week 18: Query Tools Tier 3 + Full Payment**
- Tier 3 AI-powered tools: `getReputationReport`, `analyzeDispute`, `getRootCauseAnalysis`
- x402 crypto payment integration
- Query caching with Redis
- Usage logging and metering

### Phase 6: Testing & Observability (Weeks 19-21) - SHIFTED +2

**Deliverables**:
- Comprehensive test suite (unit, integration, e2e)
- Payment integration tests
- Prometheus metrics implementation
- Grafana dashboards
- Payment monitoring and analytics
- Structured logging with tracing
- Load testing and query performance optimization

### Phase 7: Production Deployment (Weeks 22-24) - SHIFTED +2

**Deliverables**:
- CI/CD pipelines (GitHub Actions)
- Production environment configuration
- Database backups and disaster recovery
- Security audit and hardening
- API documentation (OpenAPI/Swagger)
- User guides and examples

### Phase 8: AI Integration (Week 25+)

**Deliverables**:
- Natural language trigger creation
- Event content interpretation and classification
- Trend prediction and anomaly detection
- Automated trigger optimization

## Additional Resources

### ERC-8004 Standard
- **EIP Specification**: https://eips.ethereum.org/EIPS/eip-8004
- **Contracts Repository**: https://github.com/erc-8004/erc-8004-contracts
- **Test Deployments**: See contracts repo for current testnet addresses

### OASF (Open Agentic Schema Framework)
- **Repository**: https://github.com/agntcy/oasf
- **Schema Validator**: https://oasf.agntcy.io
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

**Last Updated**: November 26, 2024
**Version**: 1.1.0
**Maintainers**: Development Team
