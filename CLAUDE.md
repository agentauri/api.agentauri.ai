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

### MCP Server (Claude Desktop Integration)
```bash
# Build MCP server
cargo build -p mcp-server --release

# Run with API key
AGENTAURI_API_KEY=sk_live_xxx ./target/release/agentauri-mcp
```

See `docs/protocols/mcp-server-claude-desktop.md` for Claude Desktop configuration.

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
- **mcp-server**: MCP server for Claude Desktop integration
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

### Public / Discovery
```
GET  /.well-known/agent.json          # A2A Agent Card
GET  /.well-known/security.txt        # Security contact
GET  /api/v1/health                   # Health check
GET  /api/v1/openapi.json             # OpenAPI spec
```

### Ponder (Indexer Status - No Auth)
```
GET  /api/v1/ponder/status            # Indexer sync status
GET  /api/v1/ponder/events            # Event statistics
```

### Authentication
```
POST /api/v1/auth/register            # Create user
POST /api/v1/auth/login               # Get JWT
POST /api/v1/auth/refresh             # Refresh access token
POST /api/v1/auth/logout              # Logout / invalidate token
GET  /api/v1/auth/me                  # Get current user info
POST /api/v1/auth/nonce               # Generate nonce for SIWE
POST /api/v1/auth/wallet              # SIWE wallet login
POST /api/v1/auth/exchange            # Exchange OAuth code for JWT
```

### Social Login (OAuth 2.0)
```
GET  /api/v1/auth/google              # Start Google OAuth
GET  /api/v1/auth/google/callback     # Google OAuth callback
GET  /api/v1/auth/github              # Start GitHub OAuth
GET  /api/v1/auth/github/callback     # GitHub OAuth callback
GET  /api/v1/auth/link/google         # Link Google to existing account
GET  /api/v1/auth/link/github         # Link GitHub to existing account
```

### OAuth Client Management
```
POST   /api/v1/oauth/token            # OAuth 2.0 token endpoint
POST   /api/v1/oauth/clients          # Create OAuth client
GET    /api/v1/oauth/clients          # List OAuth clients
DELETE /api/v1/oauth/clients/{id}     # Delete OAuth client
```

### Organizations
```
POST   /api/v1/organizations                    # Create
GET    /api/v1/organizations                    # List
GET    /api/v1/organizations/{id}               # Get
PUT    /api/v1/organizations/{id}               # Update
DELETE /api/v1/organizations/{id}               # Delete
POST   /api/v1/organizations/{id}/transfer      # Transfer ownership
```

### Organization Members
```
POST   /api/v1/organizations/{id}/members              # Add member
GET    /api/v1/organizations/{id}/members              # List members
PUT    /api/v1/organizations/{id}/members/{user_id}    # Update role
DELETE /api/v1/organizations/{id}/members/{user_id}    # Remove member
```

### Organization-Scoped Resources
```
GET    /api/v1/organizations/{id}/api-keys             # List org API keys
POST   /api/v1/organizations/{id}/api-keys             # Create org API key
GET    /api/v1/organizations/{id}/api-keys/stats       # API key statistics
GET    /api/v1/organizations/{id}/triggers             # List org triggers
GET    /api/v1/organizations/{id}/agents               # List org agents
GET    /api/v1/organizations/{id}/credits/balance      # Get org credits
GET    /api/v1/organizations/{id}/credits/transactions # List org transactions
```

### API Keys
```
POST   /api/v1/api-keys               # Create
GET    /api/v1/api-keys               # List
GET    /api/v1/api-keys/{id}          # Get
PATCH  /api/v1/api-keys/{id}          # Update
DELETE /api/v1/api-keys/{id}          # Revoke
POST   /api/v1/api-keys/{id}/rotate   # Rotate key
```

### Triggers
```
POST   /api/v1/triggers               # Create
GET    /api/v1/triggers               # List (paginated)
GET    /api/v1/triggers/{id}          # Get
PUT    /api/v1/triggers/{id}          # Update
DELETE /api/v1/triggers/{id}          # Delete
```

### Trigger Conditions
```
POST   /api/v1/triggers/{id}/conditions        # Create condition
GET    /api/v1/triggers/{id}/conditions        # List conditions
PUT    /api/v1/triggers/{id}/conditions/{cid}  # Update condition
DELETE /api/v1/triggers/{id}/conditions/{cid}  # Delete condition
```

### Trigger Actions
```
POST   /api/v1/triggers/{id}/actions           # Create action
GET    /api/v1/triggers/{id}/actions           # List actions
PUT    /api/v1/triggers/{id}/actions/{aid}     # Update action
DELETE /api/v1/triggers/{id}/actions/{aid}     # Delete action
```

### Circuit Breaker
```
GET    /api/v1/triggers/{id}/circuit-breaker       # Get state
PATCH  /api/v1/triggers/{id}/circuit-breaker       # Update config
POST   /api/v1/triggers/{id}/circuit-breaker/reset # Reset breaker
```

### Agents (Wallet Linking)
```
POST   /api/v1/agents/link            # Link agent to org
GET    /api/v1/agents/linked          # List linked agents
DELETE /api/v1/agents/{id}/link       # Unlink agent
```

### Agent Following (Simplified Monitoring)
```
GET    /api/v1/agents/following       # List followed agents
POST   /api/v1/agents/{id}/follow     # Start following agent
PUT    /api/v1/agents/{id}/follow     # Update follow settings
DELETE /api/v1/agents/{id}/follow     # Stop following agent
```

### Events (Blockchain)
```
GET    /api/v1/events                 # List events (filtered)
```

### A2A Protocol (Agent-to-Agent)
```
POST   /api/v1/a2a/rpc                # JSON-RPC 2.0 endpoint
GET    /api/v1/a2a/tasks/{id}         # Get task status
GET    /api/v1/a2a/tasks/{id}/stream  # Stream task progress (SSE)
```

### Billing
```
GET    /api/v1/billing/credits          # Get credit balance
POST   /api/v1/billing/credits/purchase # Purchase credits
GET    /api/v1/billing/transactions     # List transactions
GET    /api/v1/billing/subscription     # Get subscription info
POST   /api/v1/billing/webhook          # Stripe webhook
```

Full API docs: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`

## Testing Requirements

- ALL tests must pass before commits
- Run `./scripts/pre-push-check.sh` before pushing
- Test naming: `test_<functionality>_<scenario>_<expected_outcome>`

## Database Schema

Key tables in `database/migrations/`:

### Core
- `users` - User accounts with wallet addresses
- `organizations` - Multi-tenant organizations
- `organization_members` - Org membership and roles

### Triggers & Events
- `triggers` - Trigger definitions with circuit breaker config
- `trigger_conditions` - Trigger matching conditions
- `trigger_actions` - Actions to execute (Telegram, webhook, MCP)
- `trigger_state` - Trigger execution state
- `events` - Blockchain events (TimescaleDB hypertable)
- `action_results` - Action execution results
- `checkpoints` - Ponder indexer checkpoints
- `processed_events` - Event processing tracking

### Authentication & Security
- `api_keys` - API key storage (Argon2id hashed)
- `api_key_audit_log` - API key usage audit trail
- `auth_failures` - Brute force protection / account lockout
- `user_identities` - Social login identity mapping
- `user_refresh_tokens` - JWT refresh token storage
- `used_nonces` - SIWE nonce tracking
- `oauth_clients` - OAuth 2.0 client credentials
- `oauth_tokens` - OAuth access token storage
- `oauth_temp_codes` - OAuth authorization codes

### Billing
- `credits` - Credit balances per organization
- `credit_transactions` - Credit usage history
- `subscriptions` - Stripe subscription info
- `payment_nonces` - x402 payment nonce tracking

### Agents
- `agent_links` - Agent wallet to org linking
- `agent_mcp_tokens` - MCP authentication tokens
- `agent_follows` - Agent following for simplified monitoring

### A2A Protocol
- `a2a_tasks` - Async task tracking
- `a2a_audit_log` - A2A operation audit trail

### Ponder Integration
- `ponder_*` schema - Dynamic namespace for Ponder events
- `ponder_events_view` - Unified view across Ponder namespaces

## External References

- **ERC-8004 Spec**: https://eips.ethereum.org/EIPS/eip-8004
- **MCP Protocol**: https://modelcontextprotocol.io/docs
- **Ponder Docs**: https://ponder.sh/docs
