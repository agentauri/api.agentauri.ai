# Implementation Roadmap

> **Note**: For the most up-to-date project status, see the [README.md](../README.md) roadmap section.
> This document contains the detailed week-by-week implementation plan.

## Overview

This document outlines the complete implementation roadmap for the api.8004.dev backend infrastructure, including phases, milestones, deliverables, and subagent assignments.

## Current Status (November 2024)

**Phase 1: Foundation - ✅ COMPLETED**
- Database schema, migrations, and comprehensive test suite (108 tests)
- Security hardening and Docker infrastructure
- CI/CD pipelines with GitHub Actions
- Complete project documentation

**Phase 2: Core Services - ✅ COMPLETED (100%)**
- ✅ Rust workspace setup complete (4 crates)
- ✅ Ponder indexers fully implemented (24 handlers)
- ✅ Event Store integration complete (Week 6, 100%)
- ✅ API Gateway CRUD complete (Week 7, 100%)
- ✅ Trigger evaluation engine complete (Week 8, 100%)
- ✅ Telegram Worker complete with security hardening (Week 9, 100%)

**Phase 3.5: Payment Foundation - Week 11-12 ✅ COMPLETE**
- ✅ Organizations + Organization Members
- ✅ API Key Authentication (Layer 1) with security hardening
- ✅ Security hardening: timing attack mitigation, rate limiting, audit logging
- ✅ Credits System with atomic deduction (Week 12)
- ✅ Wallet Authentication (Layer 2) with EIP-191 verification (Week 12)
- ✅ Agent Linking with on-chain ownership verification (Week 12)
- ✅ Security fixes: race conditions, replay attacks, error sanitization (Week 12)
- ✅ HTTP client connection pooling for RPC calls (Week 12)
- 352 tests passing (verified November 28, 2024)

**Pull Layer (NEW)**
The roadmap now includes Pull Layer features for agent-initiated queries:
- Phase 3.5: Payment & Authentication Foundation (Organizations, Credits, Stripe, 3-Layer Auth)
- Phase 5 Extended: A2A Protocol + MCP Query Tools
- Timeline extended to 25 weeks (+3 weeks from original 22)

See [MCP Query Tools](api/QUERY_TOOLS.md) and [A2A Integration](protocols/A2A_INTEGRATION.md) for details.

**Authentication System (NEW)**
3-layer authentication for different client types:
- Layer 0: Anonymous (x402 only, IP-based rate limiting)
- Layer 1: API Key (`sk_live_xxx`, account-based)
- Layer 2: Wallet Signature (EIP-191, agent → account linking)

See [Authentication Documentation](../docs/auth/AUTHENTICATION.md) for details.

## Project Phases

### Phase 1: Foundation (Weeks 1-3)

**Goal**: Establish project structure, database schema, and local development environment.

#### Week 1: Project Structure & Database

**Deliverables**:
- ✅ CLAUDE.md and comprehensive documentation
- ✅ docs/ folder structure with initial documentation
- ✅ Database schema design (PostgreSQL + TimescaleDB)
- ✅ SQL migrations for all tables (9 tables, hypertables, indexes)
- ✅ Docker Compose configuration for local development
- ⏳ Rust workspace structure (next)

**Subagents**:
- ✅ `documentation-engineer` - CLAUDE.md and docs/
- ✅ `database-administrator` - Schema design and migrations
- ✅ `devops-engineer` - Docker Compose setup

**Tasks**:
1. Create all database tables (users, triggers, trigger_conditions, trigger_actions, trigger_state, events, checkpoints, action_results)
2. Set up TimescaleDB hypertables and continuous aggregates
3. Create indexes for common query patterns
4. Write initial migration files
5. Create docker-compose.yml with PostgreSQL, TimescaleDB, Redis
6. Set up Rust workspace with crates structure

#### Week 2: Shared Libraries

**Deliverables**:
- Database connection pooling utilities
- Common data models (Event, Trigger, Condition, Action)
- Error handling framework (thiserror, anyhow)
- Configuration management (environment variables, config files)
- Logging infrastructure (tracing crate)

**Subagents**:
- `rust-engineer` - Shared libraries implementation

**Tasks**:
1. Implement `shared/db` module with SQLx pool configuration
2. Define core data models in `shared/models`
3. Create error types and handling utilities
4. Implement configuration loading from .env and TOML files
5. Set up structured logging with tracing and tracing-subscriber

#### Week 3: Development Tooling

**Deliverables**:
- ✅ Local development setup scripts (`scripts/run-tests.sh`)
- ✅ Test utilities and fixtures (108 database tests)
- ✅ CI pipeline (GitHub Actions - ci.yml, security.yml, lint.yml)
- ✅ Code quality tools (security scanning, linting)

**Subagents**:
- ✅ `devops-engineer` - CI/CD setup
- ✅ `database-administrator` - Test utilities

**Tasks**:
1. ✅ Create `scripts/run-tests.sh` for comprehensive testing
2. ✅ Implement test database fixtures (test-schema.sql, test-timescaledb.sql, etc.)
3. ✅ Set up GitHub Actions for database, Rust, and TypeScript tests
4. ✅ Configure security scanning (Trivy, Gitleaks, dependency audits)
5. ⏳ Create pre-commit hooks (future)

---

### Phase 2: Event Ingestion (Weeks 4-6)

**Goal**: Implement blockchain monitoring and event storage.

#### Week 4: Ponder Indexer Setup ✅ COMPLETED

**Deliverables**:
- ✅ Ponder project structure with TypeScript
- ✅ Multi-chain configuration (Ethereum, Base, Linea, Polygon Sepolia)
- ✅ Contract ABIs for all three registries (Identity, Reputation, Validation)
- ✅ Database schema integration with existing PostgreSQL tables
- ✅ Environment-based configuration (security improvement - commit fc7a4fb)

**Subagents Used**:
- ✅ `typescript-pro` - Ponder indexers implementation (commit 287cdc8)

#### Week 5: Event Handlers ✅ COMPLETED

**Deliverables**:
- ✅ Identity Registry event handlers (AgentRegistered, MetadataUpdated)
- ✅ Reputation Registry event handlers (FeedbackSubmitted, ScoreUpdated)
- ✅ Validation Registry event handlers (ValidationPerformed, ValidationRequested)
- ✅ Event normalization and storage logic
- ✅ GraphQL API and REST endpoints (/health, /status)

**Total**: 24 event handlers (6 event types × 4 networks)

**Subagents Used**:
- ✅ `typescript-pro` - Event handler implementation

#### Week 6: Event Store Integration ✅ COMPLETED (100%)

**Deliverables**:
- ✅ PostgreSQL integration from Ponder (writes to `events` table)
- ✅ Checkpoint management per chain
- ✅ Environment-based contract address configuration
- ✅ PostgreSQL NOTIFY trigger implemented and tested
- ✅ Event Processor LISTEN verified and working
- ✅ Comprehensive test suite with 108 database tests passing
- ✅ Real-time event notification system operational

**Subagents**:
- ✅ `typescript-pro` - Ponder integration
- ✅ `database-administrator` - NOTIFY/LISTEN setup completed

#### Recent Achievements (November 2025)

**Ponder Indexers Security Hardening** (commit `6e1d19c`):
- ✅ Zod v4 environment validation with strict schema enforcement
- ✅ HTTPS-only RPC URL validation (security requirement)
- ✅ Pino structured logging with credential redaction (API keys, passwords)
- ✅ 38 unit tests with comprehensive coverage (env validation, logger)
- ✅ Pre-commit hooks via Husky (`pnpm check` validation)
- ✅ TypeScript strict mode configuration with separate `tsconfig.check.json`
- ✅ ESLint and Prettier integration with auto-fix

**Local Testing Infrastructure**:
- ✅ `local-ci.sh` - Daily development workflow validation (2-5 min)
- ✅ `local-lint.sh` - Pre-PR code quality checks (3-5 min)
- ✅ `local-security.sh` - Weekly/monthly security audit (5-10 min)
- ✅ `local-all.sh` - Complete CI/CD replication (10-15 min)

**Security Improvements**:
- ✅ All ShellCheck warnings resolved (6 scripts passing)
- ✅ Security vulnerabilities fixed (validator crate, idna dependency)
- ✅ pnpm-lock.yaml committed for reproducible builds
- ✅ CI cache paths corrected for pnpm workspace

**Code Quality**:
- ✅ All Clippy warnings resolved (0 warnings)
- ✅ Rust formatting compliant (cargo fmt)
- ✅ 31 lint checks passing (SQL, Rust, TypeScript, Docker, Shell)
- ✅ Security tools integrated (cargo-audit, trivy, gitleaks, hadolint, shellcheck)

**Testing**:
- ✅ 108 database tests passing (100% coverage)
- ✅ Event Store NOTIFY/LISTEN fully tested
- ✅ Multi-chain event processing verified
- ✅ 38 ponder-indexer unit tests (env validation + logging)

**API Gateway Implementation (Week 7)**:
- ✅ Complete REST API with 15 endpoints
- ✅ JWT authentication with Argon2 password hashing
- ✅ Repository pattern with compile-time SQL verification
- ✅ User ownership validation on all trigger operations
- ✅ Pagination support (limit/offset)
- ✅ Comprehensive API documentation (API_DOCUMENTATION.md)

---

### Phase 3: Core Backend (Weeks 7-10)

**Goal**: Implement trigger engine and basic action execution.

#### Week 7: API Gateway CRUD ✅ COMPLETED (100%)

**Deliverables**:
- ✅ Authentication endpoints (register, login with JWT)
- ✅ Triggers CRUD (5 endpoints with pagination)
- ✅ Trigger Conditions CRUD (4 endpoints)
- ✅ Trigger Actions CRUD (4 endpoints)
- ✅ JWT authentication middleware
- ✅ Argon2 password hashing
- ✅ Repository pattern with ownership validation
- ✅ Comprehensive API documentation (17KB)

**Implementation Stats**:
- 22 files changed
- 3,161 lines added
- 15 REST endpoints
- 3-layer architecture (handlers → repositories → database)

**Subagents Used**:
- ✅ `backend-architect` - API design and implementation
- ✅ `rust-engineer` - Code review and optimization

#### Week 8: Event Processor (Basic) ✅ COMPLETED

**Deliverables**:
- ✅ PostgreSQL LISTEN integration
- ✅ Trigger loading from Trigger Store
- ✅ Simple condition matching (agent_id, score_threshold, tag_equals)
- ✅ Redis job enqueueing

**Implementation Stats**:
- 34 unit tests passing
- 4 condition types implemented with comprehensive testing
- Full NOTIFY/LISTEN integration verified

**Subagents Used**:
- ✅ `rust-engineer` - Event Processor implementation

**Implemented Features**:
1. ✅ PostgreSQL NOTIFY/LISTEN connection
2. ✅ Trigger loading logic (filtered by chain_id + registry)
3. ✅ Basic condition evaluators:
   - `agent_id_equals`
   - `score_threshold` (with 6 operators: <, >, =, <=, >=, !=)
   - `tag_equals` (tag1, tag2)
   - `event_type_equals`
4. ✅ Trigger matching logic (AND conditions)
5. ✅ Redis job enqueueing for matched triggers
6. ✅ Logging and tracing integration

#### Week 9: Telegram Worker ✅ COMPLETED

**Deliverables**:
- ✅ Redis job consumption
- ✅ Telegram Bot API integration
- ✅ Message template rendering
- ✅ Retry logic and error handling
- ✅ Security hardening (12 vulnerabilities fixed)

**Implementation Stats**:
- 80 unit tests passing
- Production-ready with comprehensive security hardening
- Trait-based architecture for testability

**Subagents Used**:
- ✅ `rust-engineer` - Telegram worker implementation
- ✅ `security-engineer` - Security audit and hardening

**Implemented Features**:
1. ✅ Redis queue consumer with BRPOP
2. ✅ Teloxide Telegram Bot API integration
3. ✅ Template engine with variable substitution (25 whitelisted variables)
4. ✅ Exponential backoff retry (3 attempts: 1s, 2s, 4s)
5. ✅ Rate limiting (30 msg/sec global + per-chat)
6. ✅ Dead Letter Queue for permanent failures
7. ✅ PostgreSQL result logging
8. ✅ Prometheus metrics
9. ✅ Graceful shutdown with CancellationToken

**Security Hardening Applied**:
- ✅ Bot token protection (secrecy crate)
- ✅ Log injection prevention
- ✅ Template variable whitelist
- ✅ Chat ID validation
- ✅ Input length validation (4096 chars max)
- ✅ Job TTL (1 hour expiration)
- ✅ Per-chat rate limiting
- ✅ Safe error messages for external use

#### Week 10: Integration Testing ✅ COMPLETED

**Implemented**:
- ✅ Comprehensive test suite with 206 total tests
- ✅ 80 new api-gateway tests (from 1 to 81)
- ✅ JWT middleware tests (8 tests)
- ✅ Auth model validation tests (14 tests)
- ✅ Trigger model validation tests (17 tests)
- ✅ Condition model validation tests (11 tests)
- ✅ Action model validation tests (15 tests)
- ✅ Common model tests (pagination, responses - 16 tests)

**Test Coverage by Crate**:
| Crate | Tests | Status |
|-------|-------|--------|
| action-workers | 80 | ✅ Comprehensive |
| api-gateway | 81 | ✅ Comprehensive |
| event-processor | 34 | ✅ Good |
| shared | 11 | ✅ Good |
| **Total** | **206** | ✅ Excellent |

**Subagents**:
- `debugger` - Test implementation
- `performance-engineer` - Performance testing

---

### Phase 3.5: Payment & Authentication Foundation (Weeks 11-13) - EXTENDED +1

**Goal**: Establish multi-tenant account model, payment infrastructure, and 3-layer authentication for Pull Layer.

#### Week 11: Organizations + API Key Auth (Layer 1) ✅ COMPLETED

**Status**: Security Hardening COMPLETE (commit `26e9f92`)

**Deliverables**:
- ✅ Organizations table with multi-tenant support
- ✅ Organization members with role-based access
- ✅ Organization CRUD API endpoints
- ✅ JWT middleware updates for organization context
- ✅ **Enhanced API Keys** with `sk_live_xxx` / `sk_test_xxx` format (Layer 1 Auth)
- ✅ API Key CRUD endpoints and middleware
- ✅ **Security Hardening**:
  - Timing attack mitigation via pre-computed dummy hash for constant-time verification
  - Authentication rate limiting with Governor crate (20/min per IP, 1000/min global)
  - Dual audit logging: `api_key_audit_log` (org-scoped) + `auth_failures` (pre-org failures)

**Database Migrations**:
- ✅ `20251125000001_create_organizations_table.sql`
- ✅ `20251125000002_create_organization_members_table.sql`
- ✅ `20251126000001_create_api_keys_table.sql`
- ✅ `20251126000002_create_api_key_audit_log_table.sql`
- ✅ `20251126000003_create_auth_failures_table.sql`

**Implementation Stats**:
- 18 files changed, 3504 insertions
- 170 tests passing (8 new for rate limiter)
- DualAuth middleware (JWT + API Key)

**Subagents Used**:
- ✅ `backend-architect` - Account model design
- ✅ `rust-engineer` - Implementation
- ✅ `security-engineer` - Security hardening

**Tasks Completed**:
1. ✅ Create organizations table (id, name, slug, owner_user_id, plan, stripe_customer_id)
2. ✅ Create organization_members table with roles (admin, member, viewer)
3. ✅ Implement organization repository and handlers
4. ✅ Update JWT middleware to include organization context
5. ✅ Create API endpoints for organization CRUD and member management
6. ✅ **Create enhanced api_keys table** (environment, key_type, permissions, Argon2 hash)
7. ✅ **Implement API key generation** with secure random prefix (`sk_live_`, `sk_test_`)
8. ✅ **Create DualAuth middleware** for Layer 1 authentication
9. ✅ **Implement dual auth support** (JWT OR API Key for all endpoints)
10. ✅ **Timing attack mitigation** with pre-computed dummy hash
11. ✅ **Authentication rate limiting** with Governor crate
12. ✅ **Dual audit logging** for comprehensive security monitoring

**API Endpoints** (All Implemented):
- ✅ `POST /api/v1/organizations` - Create organization
- ✅ `GET /api/v1/organizations` - List user's organizations
- ✅ `GET /api/v1/organizations/:id` - Get organization details
- ✅ `PUT /api/v1/organizations/:id` - Update organization
- ✅ `DELETE /api/v1/organizations/:id` - Delete organization
- ✅ `POST /api/v1/organizations/:id/members` - Invite member
- ✅ `GET /api/v1/organizations/:id/members` - List members
- ✅ `DELETE /api/v1/organizations/:id/members/:user_id` - Remove member
- ✅ **`POST /api/v1/api-keys`** - Create API key
- ✅ **`GET /api/v1/api-keys`** - List organization's keys
- ✅ **`GET /api/v1/api-keys/:id`** - Get key details (masked)
- ✅ **`DELETE /api/v1/api-keys/:id`** - Revoke key
- ✅ **`POST /api/v1/api-keys/:id/rotate`** - Rotate key

#### Week 12: Credits + Wallet Auth (Layer 2) ✅ COMPLETED

**Status**: Security Hardening COMPLETE (November 27, 2024)

**Deliverables**:
- ✅ Credits table for balance tracking
- ✅ Credit transactions audit log
- ✅ Credit balance API endpoints
- ✅ Atomic credit deduction with row-level locking
- ✅ **Wallet authentication** with EIP-191 signature verification (Layer 2 Auth)
- ✅ **Agent → Account linking** with challenge-response flow
- ✅ **Nonce management** for replay attack prevention
- ✅ **On-chain ownership verification** (IdentityRegistry.ownerOf)
- ✅ **HTTP client connection pooling** for RPC calls
- ✅ **Security hardening**: error sanitization, race condition fixes

**Database Migrations**:
- ✅ `20251126000004_create_credits_table.sql`
- ✅ `20251126000005_create_credit_transactions_table.sql`
- ✅ `20251126000006_create_subscriptions_table.sql`
- ✅ `20251126000007_create_payment_nonces_table.sql`
- ✅ `20251126000008_create_agent_links_table.sql`
- ✅ `20251126000009_create_used_nonces_table.sql`

**Implementation Stats**:
- 12+ files changed
- 352 tests passing (272 api-gateway + 80 action-workers, verified November 28, 2024)
- Security vulnerabilities fixed: 7

**Security Fixes Applied**:
- ✅ **CRITICAL**: Race condition in credit deduction → Row-level locking with `SELECT ... FOR UPDATE`
- ✅ **CRITICAL**: Webhook replay attack prevention → Idempotency check via `reference_id`
- ✅ **HIGH**: Environment variable validation for Stripe keys
- ✅ **HIGH**: Nonce replay attack prevention → Database tracking with expiration
- ✅ **HIGH**: Challenge expiration validation
- ✅ **MEDIUM**: Error message sanitization (no internal details exposed)
- ✅ **QUALITY**: HTTP client connection pooling for RPC calls

**Subagents Used**:
- ✅ `backend-architect` - Payment flow design
- ✅ `rust-engineer` - Stripe and wallet verification integration
- ✅ `security-engineer` - Security audit and hardening

**Tasks Completed**:
1. ✅ Create credits table (organization_id, balance, currency)
2. ✅ Create credit_transactions table for audit log
3. ✅ Create subscriptions table for Stripe subscriptions
4. ✅ Implement credit service with atomic operations (row-level locking)
5. ✅ Create payment_nonces table for idempotency
6. ✅ Implement billing handlers and repository
7. ✅ **Add `alloy` crate** for Ethereum wallet verification
8. ✅ **Implement EIP-191 signature verification** module (`WalletService`)
9. ✅ **Create wallet challenge/verify endpoints** with nonce management
10. ✅ **Create agent_links table** (agent_id, chain_id, organization_id, wallet_address)
11. ✅ **Implement agent linking flow** with on-chain ownership verification (ownerOf call)
12. ✅ **Create used_nonces table** for replay attack prevention (5-min expiration)
13. ✅ **Implement HTTP client pooling** for RPC calls (WalletService at app startup)
14. ✅ **Security hardening** for all error responses

**API Endpoints** (All Implemented):
- ✅ `GET /api/v1/billing/credits` - Get credit balance
- ✅ `GET /api/v1/billing/transactions` - List credit transactions
- ✅ **`POST /api/v1/auth/wallet/challenge`** - Request signing challenge
- ✅ **`POST /api/v1/auth/wallet/verify`** - Submit signature, get JWT
- ✅ **`POST /api/v1/agents/link`** - Link agent to organization
- ✅ **`GET /api/v1/agents/linked`** - List linked agents
- ✅ **`DELETE /api/v1/agents/:agent_id/link`** - Unlink agent

**Dependencies Added**:
- ✅ `alloy = "0.9"` - Ethereum signature verification (EIP-191)
- ✅ `reqwest` with connection pooling - HTTP client for RPC calls

#### Week 13: Auth Completion + Rate Limiting + OAuth 2.0 ✅ COMPLETED

**Status**: COMPLETE (November 28, 2024)

**Deliverables**:
- ✅ **Layer 0 (Anonymous) IP-based rate limiting**
- ✅ **Enhanced rate limiting middleware** (per-tier, per-account, per-IP)
- ✅ **Auth layer precedence logic** (L0 < L1 < L2)
- ✅ **OAuth 2.0 tables** for future third-party integrations
- ✅ **Comprehensive auth integration tests**

**Database Migrations**:
- ✅ `20251128000010_create_oauth_clients_table.sql`
- ✅ `20251128000011_create_oauth_tokens_table.sql`

**Implementation Stats**:
- 340 tests passing (315 unit + 25 integration)
- 17 files changed in Phase 1-3 commit
- 7 files changed in Phase 4-5 commit
- 2,334 lines added in documentation
- 914 lines in integration tests

**Subagents Used**:
- ✅ `database-administrator` - OAuth 2.0 migrations
- ✅ `backend-architect` - Rate limiting architecture
- ✅ `rust-engineer` - Unified middleware implementation
- ✅ `debugger` - Integration testing
- ✅ `api-documenter` - Comprehensive documentation

**Tasks Completed**:
1. ✅ **Implement Redis-based sliding window rate limiting** (Lua script with atomic operations)
2. ✅ **Create per-tier rate limit configuration** (Anonymous: 10/hr, Free: 50/hr, Starter: 100/hr, Pro: 500/hr, Enterprise: 2000/hr)
3. ✅ **Implement IP-based rate limiting** for Layer 0 (anonymous) users with X-Forwarded-For support
4. ✅ **Create auth layer precedence extractor** (check L2 → L1 → L0 in order)
5. ✅ **Create oauth_clients table** (client_id, client_secret_hash, redirect_uris, scopes)
6. ✅ **Create oauth_tokens table** (access tokens, refresh tokens, expiration)
7. ✅ **Write integration tests** for all 3 auth layers (25 comprehensive tests)
8. ✅ **Write rate limiting tests** (IP-based, account-based, tier-based)
9. ✅ **Query tier cost multiplier tests** (Tier 0: 1x, Tier 1: 2x, Tier 2: 5x, Tier 3: 10x)
10. ✅ **Document authentication flow** in API documentation

**Architecture Components**:
- ✅ `rate_limit.lua` - Atomic Lua script for Redis (82 lines)
- ✅ `rate_limiter.rs` - RateLimiter service (434 lines)
- ✅ `auth_extractor.rs` - AuthContext extraction middleware (358 lines)
- ✅ `ip_extractor.rs` - IP extraction with proxy support (294 lines)
- ✅ `query_tier.rs` - Query tier extraction middleware (309 lines)
- ✅ `unified_rate_limiter.rs` - Unified rate limiting middleware (251 lines)
- ✅ `rate_limiting_integration.rs` - Integration tests (914 lines)

**Documentation**:
- ✅ API_DOCUMENTATION.md - Rate limiting section with examples
- ✅ AUTHENTICATION.md - Layer 0 (Anonymous) details
- ✅ QUICK_START.md - Code examples in 4 languages (curl, Python, JavaScript, Rust)
- ✅ RATE_LIMITING.md - Comprehensive rate limit rules
- ✅ ARCHITECTURE.md - Rate limiting architecture documentation
- ✅ QUICK_REFERENCE.md - Rate limiting quick reference

**API Endpoints** (OAuth 2.0 - tables ready, endpoints later):
- Tables prepared for future: `POST /oauth/authorize`, `POST /oauth/token`

**Rate Limit Configuration**:
| Layer | Auth Method | Rate Limit | Tiers Allowed | Scope |
|-------|-------------|------------|---------------|-------|
| 0 | None (IP) | 10/hour | 0-1 | IP Address |
| 1 | API Key | 50-2000/hour (plan-based) | 0-3 | Organization |
| 2 | Wallet Signature | Inherits from org | 0-3 + agent ops | Organization |

**Query Tier Cost Multipliers**:
| Tier | Description | Cost | Example Queries |
|------|-------------|------|-----------------|
| 0 | Basic | 1x | feedbacks, validations, agent profile |
| 1 | Aggregated | 2x | reputation summary, trends |
| 2 | Analysis | 5x | client analysis, baseline comparison |
| 3 | AI-powered | 10x | reputation reports, dispute analysis |

---

### Phase 4: Advanced Triggers & Actions (Weeks 14-16) - SHIFTED +1

**Goal**: Implement stateful triggers and additional action types.

#### Week 14: Stateful Triggers

**Deliverables**:
- EMA (Exponential Moving Average) condition
- Rate limit condition for triggers
- Trigger state management in PostgreSQL

**Subagents**:
- `rust-engineer` - Stateful trigger implementation

**Tasks**:
1. Implement EMA state calculation and storage
2. Create `ema_threshold` condition evaluator
3. Implement rate counter with sliding time windows
4. Create `rate_limit` condition evaluator
5. Add state update logic in Event Processor
6. Test state consistency under concurrent events

#### Week 15: REST/HTTP Worker + Discovery Endpoint

**Deliverables**:
- HTTP client integration (Reqwest)
- Support for all HTTP methods
- Request template rendering
- Response validation
- Agent Card discovery endpoint (Pull Layer)

**Subagents**:
- `backend-developer` - REST worker implementation

**Tasks**:
1. Implement Redis queue consumer for REST actions
2. Integrate Reqwest HTTP client
3. Support GET, POST, PUT, DELETE, PATCH methods
4. Implement request body and header templating
5. Add timeout configuration
6. Validate response status codes
7. Write results to Result Logger
8. Create `GET /.well-known/agent.json` endpoint for agent discovery
9. Populate agent card with service capabilities and pricing

#### Week 16: Circuit Breaker & Payment Nonces

**Deliverables**:
- Per-trigger execution rate limits
- Circuit breaker for failing triggers
- Auto-recovery mechanism
- Payment nonce generation for x402 (Pull Layer)

**Database Migrations**:
- `20250125000006_create_payment_nonces_table.sql`

**Subagents**:
- `rust-engineer` - Circuit breaker implementation

**Tasks**:
1. Implement Redis-based rate limiting (sliding window)
2. Create circuit breaker state machine
3. Auto-disable triggers with >80% failure rate
4. Implement auto-recovery after timeout
5. Add admin API for manual circuit breaker control
6. Test under failure conditions
7. Create payment_nonces table for x402 protocol
8. Implement nonce generation and validation API
9. Add x402 WWW-Authenticate headers to 402 responses

---

### Phase 5: MCP + A2A Integration (Weeks 17-19) - SHIFTED +1

**Goal**: Enable agent feedback push via MCP protocol AND agent-initiated queries via A2A/MCP (Pull Layer).

#### Week 17: MCP Bridge + A2A Protocol

**Deliverables**:
- TypeScript MCP bridge HTTP service
- MCP client integration with official SDK
- Stdio and HTTP transport support
- A2A JSON-RPC 2.0 endpoint (Pull Layer)
- A2A task lifecycle management

**Database Migrations**:
- `20250125000007_create_a2a_tasks_table.sql`
- `20250125000008_create_api_keys_table.sql`

**Subagents**:
- `typescript-pro` - MCP bridge implementation
- `mcp-developer` - Protocol compliance

**Tasks**:
1. Create standalone TypeScript service with Express
2. Integrate @modelcontextprotocol/sdk
3. Implement POST /mcp/call endpoint
4. Support stdio transport for local agents
5. Support HTTP transport for remote agents
6. Add authentication handling
7. Implement timeout and error handling
8. Create A2A JSON-RPC 2.0 endpoint (`POST /api/v1/a2a/rpc`)
9. Implement task lifecycle (submitted → working → completed)
10. Create SSE endpoint for progress updates (`GET /api/v1/a2a/tasks/:id/stream`)
11. Create api_keys table for agent authentication

**A2A Endpoints**:
- `POST /api/v1/a2a/rpc` - JSON-RPC 2.0 endpoint
- `GET /api/v1/a2a/tasks/:id` - Task status
- `GET /api/v1/a2a/tasks/:id/stream` - SSE progress updates

#### Week 18: MCP Worker + Query Tools (Tier 0-2)

**Deliverables**:
- Rust MCP worker
- Agent endpoint discovery from registration files
- IPFS file fetching and verification
- MCP bridge service integration
- MCP Query Tools Tier 0-2 (Pull Layer)

**Database Migrations**:
- `20250125000009_create_query_cache_table.sql`
- `20250125000010_create_usage_logs_table.sql`

**Subagents**:
- `rust-engineer` - MCP worker implementation
- `backend-architect` - Query tools design

**Tasks**:
1. Implement agent endpoint resolution (fetch tokenURI, parse registration file)
2. Create IPFS client integration (Pinata/Web3.Storage)
3. Implement file hash verification
4. Build MCP payload from template
5. Integrate with MCP bridge service via HTTP
6. Cache endpoint configurations
7. Handle cache invalidation on MetadataSet events
8. Implement Query Tools Tier 0 (free): `getMyFeedbacks`, `getValidationHistory`, `getAgentProfile`
9. Implement Query Tools Tier 1 (0.01 USDC): `getReputationSummary`, `getReputationTrend`
10. Implement Query Tools Tier 2 (0.05 USDC): `getClientAnalysis`, `compareToBaseline`
11. Create query_cache table for response caching
12. Create usage_logs table for billing and analytics
13. Integrate credit deduction with query execution

**MCP Query Tools (Tier 0-2)**:
| Tier | Tool | Cost | Description |
|------|------|------|-------------|
| 0 | `getMyFeedbacks` | 0.001 USDC | Raw feedback events |
| 0 | `getValidationHistory` | 0.001 USDC | Validation events |
| 0 | `getAgentProfile` | FREE | Basic agent info |
| 1 | `getReputationSummary` | 0.01 USDC | Aggregated score snapshot |
| 1 | `getReputationTrend` | 0.01 USDC | Score over time |
| 2 | `getClientAnalysis` | 0.05 USDC | Feedback patterns by client |
| 2 | `compareToBaseline` | 0.05 USDC | Compare to category average |

#### Week 19: Query Tools (Tier 3) + Full Payment

**Deliverables**:
- MCP Query Tools Tier 3 with AI analysis (Pull Layer)
- x402 crypto payment verification
- Test agent MCP server
- OASF schema validation
- End-to-end MCP tests

**Subagents**:
- `typescript-pro` - Test agent server
- `ai-engineer` - LLM integration for Tier 3
- `debugger` - MCP integration tests

**Tasks**:
1. Implement Query Tools Tier 3 (0.20 USDC): `getReputationReport`, `analyzeDispute`, `getRootCauseAnalysis`
2. Integrate LLM (Claude API) for AI-powered analysis
3. Implement x402 payment proof verification (on-chain)
4. Create test agent MCP server for development
5. Implement OASF schema validation
6. Write e2e tests: NewFeedback → MCP push → agent receives
7. Test with multiple agent configurations
8. Verify file content integrity
9. Test error scenarios (timeout, auth failure, invalid endpoint)
10. Test full payment flow (credits, Stripe, x402)

**MCP Query Tools (Tier 3)**:
| Tier | Tool | Cost | Description |
|------|------|------|-------------|
| 3 | `getReputationReport` | 0.20 USDC | Comprehensive AI analysis |
| 3 | `analyzeDispute` | 0.20 USDC | AI analysis for contested feedback |
| 3 | `getRootCauseAnalysis` | 0.20 USDC | Identify causes of anomalies |

---

### Phase 6: Testing & Observability (Weeks 20-22) - SHIFTED +1

**Goal**: Comprehensive testing and production observability including Pull Layer.

#### Week 20: Test Coverage + Auth Integration

**Deliverables**:
- Unit tests for all components (>80% coverage)
- Integration tests for cross-component flows
- Property-based tests for critical logic
- **Authentication integration tests** for all 3 layers
- Payment integration tests (Pull Layer)

**Subagents**:
- `debugger` - Test implementation

**Tasks**:
1. Write unit tests for condition evaluators
2. Write unit tests for action workers
3. Write integration tests for API Gateway → Database
4. Write integration tests for Event Processor → Queue
5. Property-based tests for EMA calculations
6. Achieve >80% code coverage
7. **Test API Key authentication flow** (Layer 1)
8. **Test wallet signature verification** (Layer 2)
9. **Test agent linking with on-chain verification**
10. **Test rate limiting per-tier enforcement**
11. Test credit deduction atomicity
12. Test Stripe payment flow
13. Test x402 payment verification
14. Test query caching behavior

#### Week 21: Observability + Auth Monitoring

**Deliverables**:
- Prometheus metrics export
- Grafana dashboards
- Structured logging with Loki
- Distributed tracing (Jaeger/Tempo)
- **Authentication metrics and alerts**
- Payment and revenue dashboards (Pull Layer)

**Subagents**:
- `devops-engineer` - Observability setup

**Tasks**:
1. Implement Prometheus metrics in all Rust services
2. Create Grafana dashboards:
   - System overview (requests, events, actions)
   - Per-chain event rates
   - Action worker performance
   - Database query performance
   - **Authentication success/failure rates by layer**
   - **Rate limit hit rates by tier**
   - **Agent linking activity**
   - Payment success rate and revenue (Pull Layer)
   - Query tool usage by tier (Pull Layer)
3. Set up Loki for log aggregation
4. Implement distributed tracing with tracing-opentelemetry
5. Create alerting rules in Prometheus
6. **Add auth failure spike alerts**
7. **Add rate limit exhaustion alerts**
8. Add payment failure alerts (Pull Layer)

#### Week 22: Load Testing + Rate Limit Performance

**Deliverables**:
- Load tests with k6 or Artillery
- Performance benchmarks
- Scalability analysis
- Optimization recommendations
- **Rate limiting performance benchmarks**
- Query tool performance benchmarks (Pull Layer)

**Subagents**:
- `performance-engineer` - Load testing and optimization

**Tasks**:
1. Create load test scenarios (1000+ events/sec)
2. Benchmark API Gateway throughput
3. Benchmark Event Processor matching latency
4. Benchmark action worker execution rates
5. Identify bottlenecks
6. Optimize critical paths
7. Document performance characteristics
8. **Benchmark API key validation latency** (target: <50ms p95)
9. **Benchmark wallet signature verification** (target: <100ms p95)
10. **Benchmark rate limit check latency** (target: <10ms p95)
11. Benchmark query tools by tier (Pull Layer)
12. Test cache hit rates and optimize (Pull Layer)
13. Load test A2A protocol (1000+ concurrent tasks)

---

### Phase 7: Production Deployment (Weeks 23-25) - SHIFTED +1

**Goal**: Production-ready deployment and documentation including Pull Layer.

#### Week 23: CI/CD Pipelines

**Deliverables**:
- GitHub Actions workflows for CI
- Automated testing on PR
- Deployment pipelines for staging and production
- Docker images for all services

**Subagents**:
- `deployment-engineer` - CI/CD implementation

**Tasks**:
1. Create .github/workflows/ci.yml (Rust tests, TypeScript tests, linting)
2. Create .github/workflows/deploy.yml (build Docker images, deploy to staging/prod)
3. Set up Docker Hub or GitHub Container Registry
4. Create production Dockerfiles (optimized, multi-stage builds)
5. Implement deployment scripts
6. Set up environment-specific configurations

#### Week 24: Security Audit

**Deliverables**:
- Security audit report
- Vulnerability fixes
- Secrets management implementation
- Security best practices documentation
- **3-layer authentication security audit**
- Payment security audit (Pull Layer)

**Subagents**:
- `security-engineer` - Security audit

**Tasks**:
1. Audit authentication and authorization
2. Review input validation and SQL injection prevention
3. Check for XSS, CSRF, and other OWASP Top 10 vulnerabilities
4. Implement secrets management (AWS Secrets Manager or HashiCorp Vault)
5. Add rate limiting and DDoS protection
6. Document security considerations
7. Fix identified vulnerabilities
8. **Audit API key generation and storage** (Argon2 hashing)
9. **Audit wallet signature verification** (EIP-191 compliance)
10. **Audit nonce management** for replay attack prevention
11. **Audit agent linking flow** for impersonation attacks
12. **Audit rate limiting** for bypass vulnerabilities
13. Audit payment processing security (Pull Layer)
14. Audit x402 verification logic (Pull Layer)
15. Secure Stripe webhook handling (Pull Layer)

#### Week 25: API Documentation & User Guides

**Deliverables**:
- OpenAPI/Swagger specification
- Postman collection
- User guides and tutorials
- Example trigger configurations
- **Authentication documentation** (docs/auth/)
- Pull Layer documentation

**Subagents**:
- `api-documenter` - API documentation
- `documentation-engineer` - User guides

**Tasks**:
1. Generate OpenAPI spec from API code
2. Create Postman collection with example requests
3. Write user guide: Getting Started
4. Write user guide: Creating Triggers
5. Write user guide: Configuring Actions
6. Create example trigger configurations (docs/examples/)
7. Write troubleshooting guide
8. **Finalize docs/auth/AUTHENTICATION.md** (3-layer overview)
9. **Finalize docs/auth/API_KEYS.md** (key management)
10. **Finalize docs/auth/WALLET_SIGNATURES.md** (EIP-191, agent linking)
11. **Finalize docs/auth/RATE_LIMITING.md** (per-tier limits)
12. **Finalize docs/auth/SECURITY_MODEL.md** (threat model)
13. Document Pull Layer API (A2A, MCP Query Tools)
14. Document payment flows (Stripe, x402, Credits)
15. Document pricing tiers and rate limits

---

### Phase 8: AI Integration (Future - Weeks 26+)

**Goal**: AI-powered trigger intelligence and event interpretation.

#### Trigger Intelligence Layer

**Deliverables**:
- Natural language to trigger conversion
- AI-assisted trigger optimization
- Smart trigger suggestions

**Subagents**:
- `ai-engineer` - AI integration

**Tasks**:
1. Integrate LLM for natural language understanding
2. Build prompt templates for trigger generation
3. Implement trigger validation and optimization
4. Create /ai/suggest-trigger API endpoint
5. Test with various natural language inputs

#### Event Interpreter

**Deliverables**:
- Feedback content summarization
- Sentiment analysis and classification
- Trend prediction

**Subagents**:
- `ai-engineer` - AI integration

**Tasks**:
1. Implement feedback file content extraction
2. Integrate LLM for summarization
3. Build sentiment classifier
4. Create severity scoring model
5. Implement trend prediction (time-series forecasting)
6. Enrich MCP payloads with AI-generated insights

---

## Milestones

### Milestone 1: MVP (End of Week 10)

**Deliverables**:
- ✅ Complete documentation (CLAUDE.md, docs/)
- Database schema and migrations
- Ponder indexers for all three registries
- API Gateway with trigger CRUD
- Event Processor with simple conditions
- Telegram worker
- Basic testing

**Success Criteria**:
- Can create triggers via API
- Can detect blockchain events from testnets
- Can send Telegram notifications when triggers match

### Milestone 1.5: Payment & Auth Foundation (End of Week 13) - EXTENDED +1

**Deliverables**:
- Multi-tenant account model (Organizations)
- Credits system with Stripe integration
- Payment webhooks and billing API
- **3-layer authentication system** (Anonymous, API Key, Wallet Signature)
- **Agent → Account linking** with on-chain verification
- **Enhanced rate limiting** (per-tier, per-account, per-IP)
- **OAuth 2.0 tables** (ready for future implementation)

**Success Criteria**:
- Can create organizations and invite members
- Can purchase credits via Stripe
- Credit balance tracking working
- **Can authenticate via API key (`sk_live_xxx`)**
- **Can authenticate via wallet signature (EIP-191)**
- **Can link agents to organizations**
- **Rate limiting enforced per authentication layer**

### Milestone 2: Full Feature Set (End of Week 19) - SHIFTED +1

**Deliverables**:
- Stateful triggers (EMA, rate limits)
- REST/HTTP worker
- MCP worker with protocol implementation
- Advanced trigger conditions
- Circuit breaker and rate limiting
- Comprehensive testing
- A2A Protocol (Pull Layer)
- MCP Query Tools Tier 0-3 (Pull Layer)
- Full payment system (Stripe + x402)

**Success Criteria**:
- Complex triggers working (EMA, rate-based)
- MCP feedback push to agents functional
- Pull Layer queries working with payment
- Production-grade error handling
- >80% test coverage

### Milestone 3: Production Ready (End of Week 25) - SHIFTED +1

**Deliverables**:
- Full observability stack
- CI/CD pipelines
- Security audit complete
- Load testing and optimization
- Complete documentation (API docs, user guides)
- Production deployment
- Payment monitoring dashboards (Pull Layer)
- A2A and MCP Query Tools documentation (Pull Layer)

**Success Criteria**:
- Deployed to production environment
- Monitoring dashboards operational
- API documentation published
- Security vulnerabilities addressed
- Performance benchmarks met
- Payment processing reliable (>99% success rate)
- Query tools performing within SLA

---

## Risk Management

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| MCP SDK compatibility issues | Medium | High | Early prototyping, contact MCP maintainers |
| IPFS gateway reliability | High | Medium | Use multiple gateways, implement caching |
| Chain reorganizations causing data inconsistency | Low | High | Thorough testing with Ponder reorg handling |
| High event volume overwhelming system | Medium | High | Load testing, horizontal scaling design |
| Postgres performance at scale | Medium | Medium | TimescaleDB optimization, read replicas |

### Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Underestimated MCP integration complexity | Medium | Medium | Add buffer time, parallel development |
| Third-party dependencies (Alchemy, Infura) downtime | Low | Medium | Multiple providers, fallback mechanisms |
| Team availability | Low | High | Clear documentation, knowledge sharing |

---

## Success Metrics

### Development Metrics

- **Code Coverage**: >80% for all Rust crates
- **Documentation Coverage**: 100% for public APIs
- **CI Success Rate**: >95%
- **Build Time**: <5 minutes for full build

### Performance Metrics

- **Event Ingestion**: >1000 events/sec per chain
- **Trigger Matching**: <100ms latency (p95)
- **Action Execution**: <5s latency (p95) for REST/Telegram, <10s for MCP
- **API Response Time**: <200ms (p95)

### Quality Metrics

- **Security Vulnerabilities**: 0 critical, 0 high
- **Uptime**: >99.9% (production)
- **Error Rate**: <0.1%
- **Failed Action Rate**: <5%

---

## Next Steps

After completing Phase 7 (Production Deployment), the project will enter maintenance mode with:

1. **Ongoing Support**: Bug fixes, security patches, dependency updates
2. **Feature Requests**: Implement user-requested features based on feedback
3. **Network Expansion**: Add support for additional blockchain networks
4. **AI Integration**: Implement Phase 8 (AI-powered features)
5. **Performance Optimization**: Continuous improvement based on production metrics

---

**Last Updated**: November 28, 2024
**Current Phase**: Phase 3.5 (Payment Foundation) - Week 11-12 ✅ COMPLETE (100%)
**Current Week**: Week 13 (Auth Completion + Rate Limiting) - Ready to start
**Next Milestone**: Week 13 - Enhanced Rate Limiting + OAuth 2.0 Tables
**Total Timeline**: 25 weeks (+3 weeks: Pull Layer +2, Authentication +1)
