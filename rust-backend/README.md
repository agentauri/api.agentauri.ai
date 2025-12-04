# Rust Backend Services

This directory contains the Rust backend services for api.agentauri.ai, implementing the core event processing, trigger matching, and action execution infrastructure.

## Architecture Overview

The backend is organized as a Cargo workspace with four main crates:

```
rust-backend/
├── Cargo.toml              # Workspace configuration
├── .cargo/
│   └── config.toml         # Build configuration
└── crates/
    ├── shared/             # Shared libraries and utilities
    ├── api-gateway/        # REST API server
    ├── event-processor/    # Trigger evaluation engine
    └── action-workers/     # Action execution workers
```

## Crates

### shared

Common functionality used across all services:

- **Database utilities**: Connection pooling with SQLx
- **Data models**: Rust structs matching PostgreSQL schema
- **Error handling**: Custom error types with thiserror
- **Configuration**: Environment variable management
- **Logging**: Structured logging with tracing

### api-gateway

REST API server built with Actix-web:

- **Authentication**: User registration/login (JWT), API Key auth (Layer 1), Wallet signatures (Layer 2)
- **Triggers CRUD**: 5 endpoints with pagination and ownership validation
- **Conditions CRUD**: 4 endpoints for trigger conditions
- **Actions CRUD**: 4 endpoints for trigger actions
- **Organizations**: Full CRUD + member management (Week 11)
- **API Keys**: Create, list, rotate, revoke (Week 11)
- **Billing**: Credit balance, transactions (Week 12)
- **Agent Linking**: Link agents to organizations with on-chain verification (Week 12)
- **Health check**: `GET /api/v1/health`
- **CORS configuration**: Environment-based whitelist
- **Middleware**: DualAuth (JWT + API Key), logging, rate limiting

See `crates/api-gateway/API_DOCUMENTATION.md` for complete API reference.

### event-processor

Listens to PostgreSQL NOTIFY events and evaluates triggers:

- **PostgreSQL LISTEN**: Receives notifications on new events
- **Trigger matching**: Evaluates events against user-defined conditions
- **Job queueing**: Enqueues matched actions to Redis
- **State management**: Handles stateful triggers (EMA, rate limits) ✅
- **Redis caching**: 8-100x faster state reads, 70-90% PostgreSQL load reduction ✅
- **Batch loading**: N+1 query optimization (66x fewer queries) ✅

See `crates/event-processor/CACHING.md` for caching implementation and `PERFORMANCE.md` for batch loading details.

### action-workers

Consumes jobs from Redis and executes actions:

- **Telegram worker**: Sends notifications via Telegram Bot API (Week 9 ✅)
- **REST worker**: Executes HTTP requests to external APIs (Planned: Week 15)
- **MCP worker**: Pushes feedback to agent MCP servers (Planned: Week 18)

## Prerequisites

- **Rust**: 1.75+ (2021 edition)
- **PostgreSQL**: 15+ with TimescaleDB extension
- **Redis**: 7+
- **Database**: Running and migrated (see `../database/`)

## Environment Variables

Create a `.env` file in the project root (one level up from `rust-backend/`):

```env
# Database
DB_HOST=localhost
DB_PORT=5432
DB_NAME=agentauri_backend
DB_USER=postgres
DB_PASSWORD=your_password_here
DB_MAX_CONNECTIONS=10

# Redis
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=your_redis_password_here

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
JWT_SECRET=your_jwt_secret_here
```

## Building

Build all crates in the workspace:

```bash
cd rust-backend
cargo build
```

Build for release (optimized):

```bash
cargo build --release
```

Build a specific crate:

```bash
cargo build -p api-gateway
cargo build -p event-processor
cargo build -p action-workers
```

## Running

### API Gateway

Start the REST API server:

```bash
cargo run -p api-gateway
```

The server will start on `http://0.0.0.0:8080`.

Test the health endpoint:

```bash
curl http://localhost:8080/api/v1/health
```

Expected response:

```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.1.0"
}
```

### Event Processor

Start the event processor:

```bash
cargo run -p event-processor
```

The processor will connect to PostgreSQL and start listening for NOTIFY events on the `new_event` channel.

### Action Workers

Start the action workers:

```bash
cargo run -p action-workers
```

The workers will connect to Redis and start consuming jobs from the action queues.

## Testing

Run all tests:

```bash
cargo test
```

Run tests for a specific crate:

```bash
cargo test -p shared
cargo test -p api-gateway
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## Development

### Code Quality

Format code:

```bash
cargo fmt
```

Run Clippy linter:

```bash
cargo clippy -- -D warnings
```

### Database Migrations

Migrations are located in `../database/migrations/` and are automatically run on service startup.

To manually run migrations:

```bash
cd ../database
./run-migrations.sh
```

### Logging

Set the `RUST_LOG` environment variable to control log levels:

```bash
# Debug logging for all services
RUST_LOG=debug cargo run -p api-gateway

# Specific log levels
RUST_LOG=shared=debug,api_gateway=info cargo run -p api-gateway
```

## Docker

Build Docker images:

```bash
# API Gateway
docker build -f ../docker/api-gateway.Dockerfile -t api-gateway .

# Event Processor
docker build -f ../docker/event-processor.Dockerfile -t event-processor .

# Action Workers
docker build -f ../docker/action-workers.Dockerfile -t action-workers .
```

## Project Status

**Phase 1: Foundation** ✅ COMPLETED

- [x] Workspace structure
- [x] Shared libraries (models, config, db, error handling)
- [x] API Gateway skeleton with health check
- [x] Event Processor skeleton with PostgreSQL LISTEN
- [x] Action Workers skeleton with Telegram and REST placeholders
- [x] All crates compile successfully

**Phase 2: Core Services** ✅ COMPLETED

- [x] Ponder indexers for blockchain events (24 handlers)
- [x] Complete API Gateway endpoints (triggers, auth)
- [x] Trigger matching logic in Event Processor
- [x] Redis job queueing
- [x] Telegram worker with security hardening

**Phase 3: Core Backend** ✅ COMPLETED

- [x] API Gateway CRUD (Week 7)
- [x] Event Processor with trigger matching (Week 8)
- [x] Telegram Worker with security (Week 9)
- [x] Integration testing (Week 10) - 206 tests

**Phase 3.5: Payment Foundation** ✅ COMPLETED

- [x] Organizations + Organization Members (Week 11)
- [x] API Key Authentication (Layer 1) with security hardening
- [x] Credits System with atomic deduction (Week 12)
- [x] Wallet Authentication (Layer 2) - EIP-191 verification
- [x] Agent Linking with on-chain ownership verification
- [x] Security hardening (race conditions, replay attacks)
- [x] **352 tests passing (272 api-gateway + 80 action-workers)**

**Phase 4: Advanced Triggers & Actions** ✅ 66% COMPLETE (Week 13-14 DONE)

- [x] Week 13: Rate Limiting Complete (Redis sliding window, 3-layer auth, 340 tests)
- [x] Week 14: Stateful Triggers (EMA, Rate Counters, 119+ tests)
- [x] Redis State Caching (8-100x faster reads, 70-90% DB load reduction)
- [x] N+1 Query Optimization (66x fewer queries)
- [ ] Week 15: REST/HTTP worker
- [ ] Week 15: Circuit breaker pattern
- [ ] Week 15: Discovery endpoint

**Total Test Suite**: 917+ tests passing across workspace

## Dependencies

Key dependencies (see `Cargo.toml` for full list):

- **actix-web** 4.10: High-performance async web framework
- **tokio** 1.42: Async runtime
- **sqlx** 0.8: Compile-time verified SQL queries
- **serde** 1.0: Serialization/deserialization
- **tracing** 0.1: Structured logging
- **redis** 0.27: Redis client
- **reqwest** 0.12: HTTP client
- **teloxide** 0.13: Telegram bot SDK

## Contributing

1. Follow Rust 2021 edition idioms
2. Use `cargo fmt` before committing
3. Ensure `cargo clippy` passes with no warnings
4. Add tests for new functionality
5. Update documentation for public APIs

## License

MIT

## References

- [CLAUDE.md](../CLAUDE.md): Complete project documentation
- [Database Schema](../database/schema.sql): PostgreSQL schema reference
- [Docker Compose](../docker-compose.yml): Local development setup
- [API Documentation](../docs/api/): API endpoint specifications (future)
