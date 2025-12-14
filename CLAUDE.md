# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Real-time backend infrastructure for monitoring ERC-8004 on-chain agent economy events. Transforms blockchain signals from Identity, Reputation, and Validation registries into automated actions (Telegram notifications, REST webhooks, MCP server updates).

## Common Commands

### Development Setup
```bash
docker-compose up -d                  # Start PostgreSQL, Redis
cargo run -p api-gateway              # Start API server (port 8080)
cargo run -p event-processor          # Start trigger processor
cargo run -p action-workers           # Start action workers
cd ponder-indexers && pnpm dev        # Start blockchain indexer
```

### Testing
```bash
# Pre-push validation (REQUIRED before pushing)
./scripts/pre-push-check.sh

# Run all Rust tests
cargo test --workspace

# Run single test (by name)
cargo test test_create_trigger

# Run tests in specific crate
cargo test -p api-gateway

# Run tests with output
cargo test -- --nocapture

# Run ignored integration tests (requires DATABASE_URL)
cargo test -- --ignored

# TypeScript tests
cd ponder-indexers && pnpm test
```

### Database Operations
```bash
sqlx migrate add <name>               # Create migration
sqlx migrate run                      # Apply migrations

# Connect to local database
PGPASSWORD="2rJ17apV8PPd1Acmg3yEfKNO62PGGsvYdHLWezqyg5U=" psql -h localhost -U postgres -d agentauri_backend

# Update SQLx offline cache (after schema changes)
DATABASE_URL="postgres://postgres:2rJ17apV8PPd1Acmg3yEfKNO62PGGsvYdHLWezqyg5U=@localhost:5432/agentauri_backend" cargo sqlx prepare --workspace
```

### Linting
```bash
cargo fmt                             # Format Rust
cargo clippy -- -D warnings           # Lint Rust
cd ponder-indexers && pnpm lint       # Lint TypeScript
```

## Architecture

```
Blockchain → Ponder Indexers → PostgreSQL → Event Processor → Redis → Action Workers → Output
                                    ↓
                             API Gateway (REST)
```

### Rust Workspace (rust-backend/)
- **api-gateway**: REST API (Actix-web), authentication, OpenAPI docs
- **event-processor**: Trigger matching, state management
- **action-workers**: Telegram, REST webhook, MCP execution
- **shared**: Database models, config, utilities

### TypeScript (ponder-indexers/)
- Ponder framework for multi-chain indexing
- Viem for Ethereum interactions
- Handlers for Identity, Reputation, Validation registries

## Key Patterns

### SQLx Offline Mode
This project uses SQLx compile-time query verification. For builds without database access:
```bash
# Use cached .sqlx metadata
SQLX_OFFLINE=true cargo build

# After changing SQL queries, regenerate cache
DATABASE_URL="postgres://postgres:2rJ17apV8PPd1Acmg3yEfKNO62PGGsvYdHLWezqyg5U=@localhost:5432/agentauri_backend" cargo sqlx prepare --workspace
```

### Authentication Layers
- **Layer 0**: Anonymous (IP rate limited)
- **Layer 1**: API Key (`sk_live_xxx`, Argon2id hashed)
- **Layer 2**: Wallet signature (EIP-191)

### OpenAPI Documentation
All API changes MUST update OpenAPI annotations:
1. Add `#[utoipa::path(...)]` to handlers
2. Add `#[derive(ToSchema)]` to DTOs
3. Register in `src/openapi.rs`

Swagger UI: http://localhost:8080/api-docs/

### Error Handling
```rust
// Library errors: use thiserror
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// Application errors: use anyhow
use anyhow::{Context, Result};
fn do_work() -> Result<()> {
    something().context("Failed to do something")?;
    Ok(())
}
```

## API Endpoints

### Authentication
```
POST /api/v1/auth/register            # Create user
POST /api/v1/auth/login               # Get JWT
GET  /api/v1/auth/google              # OAuth start
GET  /api/v1/auth/github              # OAuth start
```

### Triggers
```
GET    /api/v1/triggers               # List (paginated)
POST   /api/v1/triggers               # Create
GET    /api/v1/triggers/{id}          # Get
PUT    /api/v1/triggers/{id}          # Update
DELETE /api/v1/triggers/{id}          # Delete
```

### Organizations
```
POST   /api/v1/organizations          # Create
GET    /api/v1/organizations          # List
GET    /api/v1/organizations/:id      # Get
PUT    /api/v1/organizations/:id      # Update
DELETE /api/v1/organizations/:id      # Delete
```

### API Keys
```
POST   /api/v1/api-keys               # Create
GET    /api/v1/api-keys               # List
DELETE /api/v1/api-keys/:id           # Revoke
POST   /api/v1/api-keys/:id/rotate    # Rotate
```

Full API docs: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`

## Testing Requirements

- ALL tests must pass before commits
- Run `./scripts/pre-push-check.sh` before pushing
- Test naming: `test_<functionality>_<scenario>_<expected_outcome>`

## Database Schema

Key tables in `database/migrations/`:
- `users`, `organizations`, `organization_members`
- `triggers`, `trigger_conditions`, `trigger_actions`, `trigger_state`
- `events` (TimescaleDB hypertable)
- `api_keys`, `api_key_audit_log`
- `credits`, `credit_transactions`

## External References

- **ERC-8004 Spec**: https://eips.ethereum.org/EIPS/eip-8004
- **MCP Protocol**: https://modelcontextprotocol.io/docs
- **Ponder Docs**: https://ponder.sh/docs
