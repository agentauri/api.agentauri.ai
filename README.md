# api.8004.dev - ERC-8004 Backend Infrastructure

[![CI](https://github.com/matteoscurati/api.8004.dev/actions/workflows/ci.yml/badge.svg)](https://github.com/matteoscurati/api.8004.dev/actions/workflows/ci.yml)
[![Security Scan](https://github.com/matteoscurati/api.8004.dev/actions/workflows/security.yml/badge.svg)](https://github.com/matteoscurati/api.8004.dev/actions/workflows/security.yml)
[![Code Quality](https://github.com/matteoscurati/api.8004.dev/actions/workflows/lint.yml/badge.svg)](https://github.com/matteoscurati/api.8004.dev/actions/workflows/lint.yml)

Real-time backend infrastructure for monitoring and reacting to ERC-8004 on-chain agent economy events. Enables programmable triggers that execute automated actions based on blockchain events from Identity, Reputation, and Validation registries.

## Features

- **Multi-chain monitoring** of ERC-8004 registries across multiple networks
- **Programmable triggers** with simple, complex, and stateful conditions
- **Flexible actions**: Telegram notifications, REST webhooks, MCP server updates
- **Event store** for complete audit trail and analytics
- **Scalable architecture** with independent per-chain indexing

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

5. **Run database tests** (optional, to verify setup):
   ```bash
   ./scripts/run-tests.sh
   # Should show: All 108 tests passing
   ```

**Note**: Rust backend services and Ponder indexers are currently in development.
See the roadmap below for implementation status.

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
- [docs/protocols/](./docs/protocols/) - ERC-8004, MCP, OASF integration guides
- [docs/database/](./docs/database/) - Database schema and migration strategy
- [docs/api/](./docs/api/) - REST API specification

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

**Current Status**: ✅ 108 database tests passing (100% coverage)

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

See [docs/api/](./docs/api/) for complete API documentation.

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

### Phase 1: Foundation (Completed)
- ✅ Documentation and project structure (CLAUDE.md, README.md, docs/)
- ✅ Database schema and migrations (PostgreSQL + TimescaleDB)
- ✅ Comprehensive test suite (108 database tests, 100% coverage)
- ✅ Security hardening (environment variables, authentication, localhost binding)
- ✅ CI/CD pipelines (GitHub Actions for testing, security scanning, linting)
- ✅ Docker infrastructure (PostgreSQL, Redis, Prometheus, Grafana)

### Phase 2: Core Services (In Progress)
- 🔄 Rust workspace setup
- 🔄 Ponder indexers for all registries (Identity, Reputation, Validation)
- ⏳ API Gateway (Actix-web, authentication, rate limiting)
- ⏳ Event Processor (trigger evaluation engine)
- ⏳ Action Workers (Telegram, REST webhooks)
- ⏳ Basic trigger CRUD operations

### Phase 3: Advanced Features
- ⏳ Stateful triggers (EMA, rate limits)
- ⏳ MCP integration for agent feedback
- ⏳ WebSocket support for real-time updates
- ⏳ Advanced monitoring and alerting

### Phase 4: Production Ready
- ⏳ Load testing and performance optimization
- ⏳ Production deployment documentation
- ⏳ External security audit
- ⏳ Multi-region support

Legend: ✅ Complete | 🔄 In Progress | ⏳ Planned

---

**Built with ❤️ for the on-chain agent economy**
