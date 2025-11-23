# api.8004.dev - ERC-8004 Backend Infrastructure

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
   git clone https://github.com/your-org/api.8004.dev.git
   cd api.8004.dev
   ```

2. **Start infrastructure** (PostgreSQL, Redis):
   ```bash
   docker-compose up -d
   ```

3. **Run database migrations**:
   ```bash
   cd rust-backend
   sqlx migrate run
   ```

4. **Start Ponder indexers**:
   ```bash
   cd ponder-indexers
   pnpm install
   pnpm dev
   ```

5. **Start API Gateway**:
   ```bash
   cd rust-backend/crates/api-gateway
   cargo run
   ```

6. **Start Event Processor**:
   ```bash
   cd rust-backend/crates/event-processor
   cargo run
   ```

7. **Start Action Workers**:
   ```bash
   cd rust-backend/crates/action-workers
   cargo run
   ```

The API will be available at `http://localhost:8000`.

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

```bash
# Rust tests
cd rust-backend
cargo test

# TypeScript tests
cd ponder-indexers
pnpm test
```

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

## Support

- **Documentation**: [CLAUDE.md](./CLAUDE.md)
- **Issues**: [GitHub Issues](https://github.com/your-org/api.8004.dev/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/api.8004.dev/discussions)

## Related Projects

- **ERC-8004 Standard**: https://eips.ethereum.org/EIPS/eip-8004
- **ERC-8004 Contracts**: https://github.com/erc-8004/erc-8004-contracts
- **OASF**: https://github.com/agntcy/oasf
- **MCP Specification**: https://github.com/modelcontextprotocol/specification

## Roadmap

See [docs/ROADMAP.md](./docs/ROADMAP.md) for detailed development timeline and milestones.

### Phase 1: MVP (Weeks 1-8)
- ✅ Documentation and project structure
- Database schema and migrations
- Ponder indexers for all registries
- Basic trigger engine
- Telegram worker

### Phase 2: Full Feature Set (Weeks 9-12)
- Stateful triggers (EMA, rate limits)
- REST/HTTP worker
- MCP integration

### Phase 3: Production Ready (Weeks 13-16)
- Observability stack
- Load testing
- CI/CD pipelines
- Security audit

---

**Built with ❤️ for the on-chain agent economy**
