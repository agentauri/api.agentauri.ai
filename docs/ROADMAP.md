# Implementation Roadmap

> **Note**: For the most up-to-date project status, see the [README.md](../README.md) roadmap section.
> This document contains the detailed week-by-week implementation plan.

## Overview

This document outlines the complete implementation roadmap for the api.8004.dev backend infrastructure, including phases, milestones, deliverables, and subagent assignments.

## Current Status (November 2025)

**Phase 1: Foundation - ✅ COMPLETED**
- Database schema, migrations, and comprehensive test suite (108 tests)
- Security hardening and Docker infrastructure
- CI/CD pipelines with GitHub Actions
- Complete project documentation

**Phase 2: Core Services - 🔄 IN PROGRESS**
- Starting Rust workspace and Ponder indexers setup

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

#### Week 4: Ponder Indexer Setup

**Deliverables**:
- Ponder project structure
- Multi-chain configuration
- Contract ABIs for all three registries

**Subagents**:
- `typescript-pro` - Ponder setup and configuration

**Tasks**:
1. Initialize Ponder project with TypeScript
2. Configure networks (Ethereum Sepolia, Base Sepolia, Linea Sepolia, Polygon Amoy)
3. Fetch and store contract ABIs from erc-8004-contracts repo
4. Set up RPC provider connections (Alchemy, Infura)
5. Configure ponder.config.ts with contract addresses

#### Week 5: Event Handlers

**Deliverables**:
- IdentityRegistry event handlers (Registered, MetadataSet)
- ReputationRegistry event handlers (NewFeedback, FeedbackRevoked, ResponseAppended)
- ValidationRegistry event handlers (ValidationRequest, ValidationResponse)
- Event normalization logic

**Subagents**:
- `typescript-pro` - Event handler implementation

**Tasks**:
1. Implement `onAgentRegistered` handler
2. Implement `onMetadataSet` handler
3. Implement `onNewFeedback` handler with tag decoding
4. Implement `onFeedbackRevoked` handler
5. Implement `onValidationRequest` handler
6. Implement `onValidationResponse` handler with tag decoding
7. Create event normalization utilities (bytes32 to string conversion)

#### Week 6: Event Store Integration

**Deliverables**:
- PostgreSQL integration from Ponder
- Checkpoint management
- Reorg handling
- Event Store query utilities

**Subagents**:
- `typescript-pro` - PostgreSQL integration
- `database-administrator` - Query optimization

**Tasks**:
1. Configure Ponder to write to PostgreSQL Event Store
2. Implement checkpoint updates on each block processed
3. Test chain reorganization handling
4. Create utility functions for querying events
5. Set up PostgreSQL NOTIFY triggers
6. Performance testing with high event volume

---

### Phase 3: Core Backend (Weeks 7-10)

**Goal**: Implement trigger engine and basic action execution.

#### Week 7: API Gateway

**Deliverables**:
- REST API server (Actix-web)
- JWT authentication
- User registration and login endpoints
- Trigger CRUD endpoints

**Subagents**:
- `backend-architect` - API design
- `backend-developer` - API implementation

**Tasks**:
1. Set up Actix-web server with routing
2. Implement JWT authentication middleware
3. Create POST /api/v1/auth/register endpoint
4. Create POST /api/v1/auth/login endpoint
5. Implement trigger CRUD endpoints (POST, GET, PUT, DELETE /api/v1/triggers)
6. Add input validation with validator crate
7. Implement error handling and response formatting

#### Week 8: Event Processor (Basic)

**Deliverables**:
- PostgreSQL LISTEN integration
- Trigger loading from Trigger Store
- Simple condition matching (agent_id, score_threshold, tag_equals)
- Redis job enqueueing

**Subagents**:
- `rust-engineer` - Event Processor implementation

**Tasks**:
1. Implement PostgreSQL NOTIFY/LISTEN connection
2. Create trigger loading logic (filtered by chain_id + registry)
3. Implement basic condition evaluators:
   - `agent_id_equals`
   - `score_threshold`
   - `tag_equals`
   - `event_type_equals`
4. Implement trigger matching logic (AND conditions)
5. Create Redis job enqueueing for matched triggers
6. Add logging and metrics

#### Week 9: Telegram Worker

**Deliverables**:
- Redis job consumption
- Telegram Bot API integration
- Message template rendering
- Retry logic and error handling

**Subagents**:
- `backend-developer` - Telegram worker implementation

**Tasks**:
1. Implement Redis queue consumer
2. Integrate Teloxide or direct Telegram Bot API
3. Create message template engine with variable substitution
4. Implement retry logic with exponential backoff
5. Handle Telegram rate limits
6. Write action results to Result Logger
7. Add Prometheus metrics

#### Week 10: Integration Testing

**Deliverables**:
- End-to-end tests for basic trigger flow
- Integration tests for API endpoints
- Performance benchmarks

**Subagents**:
- `debugger` - Test implementation
- `performance-engineer` - Performance testing

**Tasks**:
1. Write e2e tests: event → trigger match → Telegram notification
2. Test API endpoints with authentication
3. Test trigger CRUD operations
4. Benchmark event processing throughput
5. Identify and fix performance bottlenecks

---

### Phase 4: Advanced Triggers & Actions (Weeks 11-13)

**Goal**: Implement stateful triggers and additional action types.

#### Week 11: Stateful Triggers

**Deliverables**:
- EMA (Exponential Moving Average) condition
- Rate limit condition
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

#### Week 12: REST/HTTP Worker

**Deliverables**:
- HTTP client integration (Reqwest)
- Support for all HTTP methods
- Request template rendering
- Response validation

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

#### Week 13: Circuit Breaker & Rate Limiting

**Deliverables**:
- Per-trigger execution rate limits
- Circuit breaker for failing triggers
- Auto-recovery mechanism

**Subagents**:
- `rust-engineer` - Circuit breaker implementation

**Tasks**:
1. Implement Redis-based rate limiting (sliding window)
2. Create circuit breaker state machine
3. Auto-disable triggers with >80% failure rate
4. Implement auto-recovery after timeout
5. Add admin API for manual circuit breaker control
6. Test under failure conditions

---

### Phase 5: MCP Integration (Weeks 14-16)

**Goal**: Enable agent feedback push via MCP protocol.

#### Week 14: MCP Bridge Service

**Deliverables**:
- TypeScript MCP bridge HTTP service
- MCP client integration with official SDK
- Stdio and HTTP transport support

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

#### Week 15: MCP Worker

**Deliverables**:
- Rust MCP worker
- Agent endpoint discovery from registration files
- IPFS file fetching and verification
- MCP bridge service integration

**Subagents**:
- `rust-engineer` - MCP worker implementation

**Tasks**:
1. Implement agent endpoint resolution (fetch tokenURI, parse registration file)
2. Create IPFS client integration (Pinata/Web3.Storage)
3. Implement file hash verification
4. Build MCP payload from template
5. Integrate with MCP bridge service via HTTP
6. Cache endpoint configurations
7. Handle cache invalidation on MetadataSet events

#### Week 16: MCP Testing & OASF Integration

**Deliverables**:
- Test agent MCP server
- OASF schema validation
- End-to-end MCP tests

**Subagents**:
- `typescript-pro` - Test agent server
- `debugger` - MCP integration tests

**Tasks**:
1. Create test agent MCP server for development
2. Implement OASF schema validation
3. Write e2e tests: NewFeedback → MCP push → agent receives
4. Test with multiple agent configurations
5. Verify file content integrity
6. Test error scenarios (timeout, auth failure, invalid endpoint)

---

### Phase 6: Testing & Observability (Weeks 17-19)

**Goal**: Comprehensive testing and production observability.

#### Week 17: Test Coverage

**Deliverables**:
- Unit tests for all components (>80% coverage)
- Integration tests for cross-component flows
- Property-based tests for critical logic

**Subagents**:
- `debugger` - Test implementation

**Tasks**:
1. Write unit tests for condition evaluators
2. Write unit tests for action workers
3. Write integration tests for API Gateway → Database
4. Write integration tests for Event Processor → Queue
5. Property-based tests for EMA calculations
6. Achieve >80% code coverage

#### Week 18: Observability Stack

**Deliverables**:
- Prometheus metrics export
- Grafana dashboards
- Structured logging with Loki
- Distributed tracing (Jaeger/Tempo)

**Subagents**:
- `devops-engineer` - Observability setup

**Tasks**:
1. Implement Prometheus metrics in all Rust services
2. Create Grafana dashboards:
   - System overview (requests, events, actions)
   - Per-chain event rates
   - Action worker performance
   - Database query performance
3. Set up Loki for log aggregation
4. Implement distributed tracing with tracing-opentelemetry
5. Create alerting rules in Prometheus

#### Week 19: Load Testing

**Deliverables**:
- Load tests with k6 or Artillery
- Performance benchmarks
- Scalability analysis
- Optimization recommendations

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

---

### Phase 7: Production Deployment (Weeks 20-22)

**Goal**: Production-ready deployment and documentation.

#### Week 20: CI/CD Pipelines

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

#### Week 21: Security Audit

**Deliverables**:
- Security audit report
- Vulnerability fixes
- Secrets management implementation
- Security best practices documentation

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

#### Week 22: API Documentation & User Guides

**Deliverables**:
- OpenAPI/Swagger specification
- Postman collection
- User guides and tutorials
- Example trigger configurations

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

---

### Phase 8: AI Integration (Future - Weeks 23+)

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

### Milestone 2: Full Feature Set (End of Week 16)

**Deliverables**:
- Stateful triggers (EMA, rate limits)
- REST/HTTP worker
- MCP worker with protocol implementation
- Advanced trigger conditions
- Circuit breaker and rate limiting
- Comprehensive testing

**Success Criteria**:
- Complex triggers working (EMA, rate-based)
- MCP feedback push to agents functional
- Production-grade error handling
- >80% test coverage

### Milestone 3: Production Ready (End of Week 22)

**Deliverables**:
- Full observability stack
- CI/CD pipelines
- Security audit complete
- Load testing and optimization
- Complete documentation (API docs, user guides)
- Production deployment

**Success Criteria**:
- Deployed to production environment
- Monitoring dashboards operational
- API documentation published
- Security vulnerabilities addressed
- Performance benchmarks met

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

**Last Updated**: January 23, 2025
**Current Phase**: Phase 1 (Foundation)
**Current Week**: Week 1
**Next Milestone**: MVP (End of Week 10)
