# api.8004.dev - ERC-8004 Backend Infrastructure

[![CI](https://github.com/matteoscurati/api.8004.dev/actions/workflows/ci.yml/badge.svg)](https://github.com/matteoscurati/api.8004.dev/actions/workflows/ci.yml)
[![Security Scan](https://github.com/matteoscurati/api.8004.dev/actions/workflows/security.yml/badge.svg)](https://github.com/matteoscurati/api.8004.dev/actions/workflows/security.yml)
[![Code Quality](https://github.com/matteoscurati/api.8004.dev/actions/workflows/lint.yml/badge.svg)](https://github.com/matteoscurati/api.8004.dev/actions/workflows/lint.yml)

Real-time backend infrastructure for monitoring and reacting to ERC-8004 on-chain agent economy events. Enables programmable triggers that execute automated actions based on blockchain events from Identity, Reputation, and Validation registries.

## Features

**PUSH Layer** (event-driven):
- **Multi-chain monitoring** of ERC-8004 registries across multiple networks
- **Programmable triggers** with simple, complex, and stateful conditions
- **Flexible actions**: Telegram notifications, REST webhooks, MCP server updates
- **Event store** for complete audit trail and analytics
- **Scalable architecture** with independent per-chain indexing

**PULL Layer** (agent-initiated queries):
- **A2A Protocol** (Google Agent-to-Agent) for async task-based queries
- **MCP Query Tools** (4 tiers): raw queries, aggregations, analysis, AI-powered insights
- **Payment System**: Stripe (fiat), x402 (crypto), Credits (prepaid)
- **Multi-tenant Account Model** with organizations and role-based access

**Authentication System** (3-layer):
- **Layer 0**: Anonymous access with x402 micropayments, IP-based rate limiting
- **Layer 1**: API Key authentication (`sk_live_xxx`), per-plan rate limits
- **Layer 2**: Wallet signature (EIP-191), agent → account linking
- **Social Login**: Google and GitHub OAuth 2.0 with automatic account linking
- **Account Security**: Progressive lockout after failed attempts (15min → 4h)

## Quick Start

### Prerequisites

- **Rust** 1.75+ (for backend services)
- **Node.js** 20+ (for Ponder indexers and MCP bridge)
- **PostgreSQL** 15+ with TimescaleDB extension
- **Redis** 7+
- **Docker & Docker Compose** (for local development)

### Local Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/matteoscurati/api.8004.dev.git
   cd api.8004.dev
   ```

2. **Configure environment variables**:
   ```bash
   cp .env.example .env
   # Edit .env and set secure passwords (use: openssl rand -base64 32)
   ```

3. **Start infrastructure** (PostgreSQL + TimescaleDB, Redis, Prometheus, Grafana):
   ```bash
   docker-compose up -d
   ```

4. **Verify services are running**:
   ```bash
   docker-compose ps
   # All services should show "Up" and "healthy" status
   ```

5. **Run tests** (optional, to verify setup):
   ```bash
   ./scripts/run-tests.sh
   # Should show all test suites passing
   ```

**Note**: Phase 3, Phase 3.5, and Phase 4 are complete. OpenAPI documentation with Swagger UI is available at `/api-docs/`. See roadmap for details.

### 6. Run Rust Services (optional)

The Rust backend services are working skeletons ready for development:

```bash
# Terminal 1: API Gateway (REST API server)
cd rust-backend
cargo run -p api-gateway
# Runs on http://localhost:8080
# Health check: curl http://localhost:8080/api/v1/health

# Terminal 2: Event Processor (trigger evaluation)
cargo run -p event-processor
# Listens to PostgreSQL NOTIFY on 'new_event' channel

# Terminal 3: Action Workers (execute actions)
cargo run -p action-workers
# Processes jobs from Redis queue
```

### 7. Run Ponder Indexers (optional)

```bash
cd ponder-indexers
pnpm dev
# Indexes blockchain events on 4 testnets
# API at http://localhost:42069
# Note: Requires RPC URLs and contract addresses in .env
```

## Architecture

The system consists of 9 core components:

```
Blockchain → RPC Nodes → Ponder Indexers → Event Store → Event Processor → Action Workers → Output Channels
                                              ↓
                                         Trigger Store
```

See [CLAUDE.md](./CLAUDE.md) for comprehensive architecture documentation.

## Tech Stack

- **Rust**: Actix-web, Tokio, SQLx, Reqwest (backend services)
- **TypeScript**: Ponder, Viem (blockchain indexing)
- **PostgreSQL + TimescaleDB**: Event and trigger storage
- **Redis**: Job queuing
- **MCP Protocol**: Agent communication

## Documentation

- [CLAUDE.md](./CLAUDE.md) - Complete project documentation
- [docs/architecture/](./docs/architecture/) - System architecture and diagrams
- [docs/protocols/](./docs/protocols/) - ERC-8004, MCP, OASF, A2A integration guides
- [docs/auth/](./docs/auth/) - Authentication system (API keys, wallet signatures, social login)
- [docs/auth/SOCIAL_LOGIN.md](./docs/auth/SOCIAL_LOGIN.md) - Google & GitHub OAuth 2.0 setup
- [docs/database/](./docs/database/) - Database schema and migration strategy
- [API Documentation](./rust-backend/crates/api-gateway/API_DOCUMENTATION.md) - REST API specification
- **Swagger UI**: Available at `/api-docs/` when running the API gateway

## Development

### Running Tests

The project enforces **100% test coverage** before commits. All tests must pass for CI/CD to succeed.

```bash
# Run all tests (database, Rust, TypeScript)
./scripts/run-tests.sh

# Database tests only (108 tests covering schema, TimescaleDB, integrity, notifications, performance)
docker exec -i erc8004-postgres psql -U postgres -d erc8004_backend < database/tests/test-schema.sql
docker exec -i erc8004-postgres psql -U postgres -d erc8004_backend < database/tests/test-timescaledb.sql
docker exec -i erc8004-postgres psql -U postgres -d erc8004_backend < database/tests/test-data-integrity.sql
docker exec -i erc8004-postgres psql -U postgres -d erc8004_backend < database/tests/test-notifications.sql
docker exec -i erc8004-postgres psql -U postgres -d erc8004_backend < database/tests/test-performance-simple.sql

# Rust tests (when implemented)
cd rust-backend
cargo test

# TypeScript tests (when implemented)
cd ponder-indexers
pnpm test
```

**Current Status**: ✅ 170+ Rust tests passing, 40+ database tests, comprehensive coverage across all crates

### Local Testing

Replicate GitHub Actions CI/CD checks locally before pushing. All scripts are located in `scripts/` and provide colored output with detailed feedback.

#### Quick Reference

```bash
# Daily development workflow (fastest, 2-5 min)
./scripts/local-ci.sh

# Pre-PR code quality checks (3-5 min)
./scripts/local-lint.sh

# Security audit - run weekly/monthly (5-10 min)
./scripts/local-security.sh

# Complete CI replication - run before pushing to main (10-15 min)
./scripts/local-all.sh
```

#### Script Details

**`local-ci.sh` - Daily Development Workflow**
- Database tests (schema, TimescaleDB, integrity, notifications, performance)
- Rust tests (formatting, Clippy, build, unit tests)
- TypeScript tests (type-check, linting, tests)

**`local-lint.sh` - Pre-PR Code Quality**
- SQL linting (style consistency, trailing whitespace)
- Rust linting (formatting, Clippy warnings, unsafe code, TODO comments)
- TypeScript linting (formatting, ESLint, type checking)
- Documentation checks (required files, broken links)
- Docker Compose validation
- Shell script linting with ShellCheck (if installed)

**`local-security.sh` - Security Audit**
- Dependency vulnerability scanning (cargo-audit, npm audit)
- Docker image security (Trivy)
- Secrets detection (Gitleaks)
- Dockerfile linting (hadolint)
- Configuration security checks (JWT, CORS, .env)

**`local-all.sh` - Complete CI Replication**
- Runs all three scripts in sequence
- Provides comprehensive summary with timing
- Pass `--yes` or `-y` to skip confirmation prompt

#### Optional Security Tools

For complete security coverage, install these tools:

```bash
# Rust dependency audit
cargo install cargo-audit

# macOS (Homebrew)
brew install trivy gitleaks hadolint shellcheck

# Ubuntu/Debian
apt install shellcheck
# (trivy, gitleaks, hadolint: see official installation docs)
```

Scripts will gracefully skip checks if tools aren't installed.

#### Exit Codes

All scripts follow GitHub Actions conventions:
- `0`: All checks passed
- `1`: Critical failures found
- Some scripts may exit `0` with warnings (review output)

### Code Style

```bash
# Format Rust code
cargo fmt

# Format TypeScript code
cd ponder-indexers
pnpm format

# Lint
cargo clippy
pnpm lint
```

## API Usage

### Create a Trigger

```bash
POST /api/v1/triggers
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Low Score Alert",
  "organization_id": "org_xxxxx",
  "chain_id": 84532,
  "registry": "reputation",
  "conditions": [
    {
      "condition_type": "agent_id_equals",
      "field": "agent_id",
      "operator": "=",
      "value": "42"
    },
    {
      "condition_type": "score_threshold",
      "field": "score",
      "operator": "<",
      "value": "60"
    }
  ],
  "actions": [
    {
      "action_type": "telegram",
      "priority": 1,
      "config": {
        "chat_id": "123456789",
        "message_template": "Agent #{{agent_id}} received low score: {{score}}/100"
      }
    }
  ]
}
```

See [API Documentation](./rust-backend/crates/api-gateway/API_DOCUMENTATION.md) for complete API documentation.

## Supported Networks

### Testnet (Initial Release)
- Ethereum Sepolia
- Base Sepolia
- Linea Sepolia
- Polygon Amoy

### Mainnet (Planned)
- Ethereum
- Base
- Arbitrum
- Optimism

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run linters and tests
6. Submit a pull request

## License

MIT License - see [LICENSE](./LICENSE) for details.

## Security

This project implements comprehensive security best practices:

- **Environment Variables**: All sensitive credentials stored in `.env` (never committed)
- **Strong Passwords**: Generated with `openssl rand -base64 32`
- **Authentication Required**: Redis, PostgreSQL, and Grafana all require authentication
- **Localhost Binding**: All ports bound to `127.0.0.1` to prevent external access
- **Pinned Dependencies**: Docker images pinned to specific versions
- **Resource Limits**: Container resource limits to prevent DoS attacks
- **Security Scanning**: Automated vulnerability scanning via GitHub Actions
- **Regular Audits**: Weekly security scans with Trivy, Gitleaks, and dependency audits

See `.env.example` for configuration guidelines.

## Support

- **Documentation**: [CLAUDE.md](./CLAUDE.md)
- **Issues**: [GitHub Issues](https://github.com/matteoscurati/api.8004.dev/issues)
- **Discussions**: [GitHub Discussions](https://github.com/matteoscurati/api.8004.dev/discussions)

## Related Projects

- **ERC-8004 Standard**: https://eips.ethereum.org/EIPS/eip-8004
- **ERC-8004 Contracts**: https://github.com/erc-8004/erc-8004-contracts
- **OASF**: https://github.com/agntcy/oasf
- **MCP Specification**: https://github.com/modelcontextprotocol/specification

## Roadmap

See [docs/ROADMAP.md](./docs/ROADMAP.md) for detailed development timeline and milestones.

**Total Timeline**: 24 weeks (Phases 1-7)

### Phase 1-2: Foundation & Event Ingestion (Weeks 1-7) - ✅ COMPLETE
- ✅ Documentation and project structure (CLAUDE.md, README.md, docs/)
- ✅ Database schema and migrations (PostgreSQL + TimescaleDB)
- ✅ Comprehensive test suite (108 database tests, 100% coverage)
- ✅ Ponder indexers (24 event handlers across 4 networks)
- ✅ API Gateway CRUD endpoints (15 endpoints, JWT auth)

### Phase 3: Core Backend (Weeks 8-10) - ✅ COMPLETE
- ✅ Event Processor with trigger matching
- ✅ Telegram Worker with security hardening
- ✅ Integration Testing (206 total tests across workspace)

### Phase 3.5: Payment Foundation (Weeks 11-12) - ✅ COMPLETE
- ✅ Organizations & multi-tenant account model (database + API)
- ✅ Credits system & Stripe integration (Week 12)
- ✅ Wallet Authentication Layer 2 (EIP-191, Week 12)
- ✅ Agent Linking with on-chain verification (Week 12)
- ✅ API Key Authentication Layer 1 with security hardening (Week 11)
- ✅ Role-based access (admin, member, viewer)

### Phase 4: Advanced Triggers & Actions (Weeks 13-15) - ✅ COMPLETE
- ✅ Auth Completion + Rate Limiting + OAuth 2.0 (Week 13)
- ✅ Stateful Triggers (EMA + Rate Counters) (Week 14)
- ✅ Social Login (Google + GitHub OAuth 2.0)
- ✅ Account Lockout (progressive: 15min → 4h)
- ✅ Circuit Breaker for trigger resilience
- ✅ Discovery endpoint (`/.well-known/agent.json`)
- ✅ OpenAPI 3.0 documentation with Swagger UI (`/api-docs/`)

### Phase 5: MCP + A2A Integration (Weeks 16-18)
- ⏳ A2A Protocol (Google Agent-to-Agent)
- ⏳ MCP Query Tools (Tier 0-3)
- ⏳ x402 crypto payment integration
- ⏳ Query caching and usage metering

### Phase 6: Testing & Observability (Weeks 19-21)
- ⏳ Comprehensive test suite (unit, integration, e2e)
- ⏳ Payment integration tests
- ⏳ Prometheus metrics & Grafana dashboards

### Phase 7: Production Deployment (Weeks 22-24)
- ⏳ CI/CD pipelines
- ⏳ Security audit and hardening
- ✅ API documentation (OpenAPI/Swagger) - completed in Phase 4

### Phase 8: AI Integration (Week 25+)
- ⏳ Natural language trigger creation
- ⏳ Trend prediction and anomaly detection

Legend: ✅ Complete | 🔄 In Progress | ⏳ Planned

---

**Built with ❤️ for the on-chain agent economy**
