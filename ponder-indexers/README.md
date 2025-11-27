# Ponder Indexers for ERC-8004 Registries

Real-time blockchain event indexing for ERC-8004 Identity, Reputation, and Validation registries across multiple chains.

## Overview

This Ponder project indexes events from three ERC-8004 registry types across four testnets:

**Registries:**
- Identity Registry (AgentRegistered, MetadataUpdated)
- Reputation Registry (FeedbackSubmitted, ScoreUpdated)
- Validation Registry (ValidationPerformed, ValidationRequested)

**Networks:**
- Ethereum Sepolia (Chain ID: 11155111)
- Base Sepolia (Chain ID: 84532)
- Linea Sepolia (Chain ID: 59141)
- Polygon Amoy (Chain ID: 80002)

## Prerequisites

- Node.js 20+
- pnpm 8+
- PostgreSQL 15+ (running via Docker from root project)
- RPC API keys for each network

## Quick Start

### 1. Install Dependencies

```bash
pnpm install
```

### 2. Configure Environment

```bash
cp .env.example .env
# Edit .env and add your RPC API keys
```

Required environment variables in `.env`:

**RPC URLs:**
- `ETHEREUM_SEPOLIA_RPC_URL`
- `BASE_SEPOLIA_RPC_URL`
- `LINEA_SEPOLIA_RPC_URL`
- `POLYGON_AMOY_RPC_URL`

**Database:**
- `DATABASE_URL` (same as Rust backend)

**Contract Addresses** (12 total - see `.env.example`):
- `ETHEREUM_SEPOLIA_IDENTITY_ADDRESS`
- `ETHEREUM_SEPOLIA_REPUTATION_ADDRESS`
- `ETHEREUM_SEPOLIA_VALIDATION_ADDRESS`
- `ETHEREUM_SEPOLIA_START_BLOCK`
- (+ 8 more for Base, Linea, Polygon - see [CONTRACTS.md](./CONTRACTS.md))

### 3. Configure Contract Addresses

Update the contract addresses in your `.env` file. See [CONTRACTS.md](./CONTRACTS.md) for detailed instructions.

Example:
```bash
ETHEREUM_SEPOLIA_IDENTITY_ADDRESS=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb
ETHEREUM_SEPOLIA_REPUTATION_ADDRESS=0x8F2E097E79B1c51Be9cA9dF1c8B5aC2b7ddEEd20
ETHEREUM_SEPOLIA_VALIDATION_ADDRESS=0x9D4E94dB8EfBa94BdBABFC33B7e45e4E5c5e5e5e
ETHEREUM_SEPOLIA_START_BLOCK=5000000
```

⚠️ **Important:** Set `START_BLOCK` to the deployment block number for faster initial sync.

### 4. Run the Indexer

Development mode (with hot reload):
```bash
pnpm dev
```

Production mode:
```bash
pnpm build
pnpm start
```

## Project Structure

```
ponder-indexers/
├── abis/                           # Contract ABIs
│   ├── IdentityRegistry.json      # Identity Registry ABI
│   ├── ReputationRegistry.json    # Reputation Registry ABI
│   └── ValidationRegistry.json    # Validation Registry ABI
├── src/
│   ├── index.ts                   # Event handlers
│   ├── env-validation.ts          # Zod environment validation
│   ├── logger.ts                  # Pino structured logging
│   ├── api/
│   │   └── index.ts               # GraphQL API extensions
│   └── __tests__/                 # Unit tests
│       ├── env-validation.test.ts # Environment validation tests
│       └── logger.test.ts         # Logger tests
├── ponder.config.ts               # Ponder configuration
├── ponder.schema.ts               # Database schema
├── ponder-env.d.ts                # Type definitions
├── tsconfig.json                  # TypeScript configuration
├── tsconfig.check.json            # Type checking configuration
├── package.json                   # Dependencies and scripts
├── .husky/                        # Git hooks
│   └── pre-commit                 # Pre-commit validation
└── README.md                      # This file
```

## Event Handlers

### Identity Registry Events

**AgentRegistered**
- Triggered when a new agent is registered
- Stores: agentId, owner, tokenURI, timestamp

**MetadataUpdated**
- Triggered when agent metadata is updated
- Stores: agentId, key, value, timestamp

### Reputation Registry Events

**FeedbackSubmitted**
- Triggered when feedback is submitted for an agent
- Stores: agentId, client, feedbackIndex, score, tags, fileURI, fileHash, timestamp

**ScoreUpdated**
- Triggered when an agent's reputation score changes
- Stores: agentId, newScore, feedbackCount, timestamp

### Validation Registry Events

**ValidationPerformed**
- Triggered when a validation is completed
- Stores: agentId, validator, requestHash, response, responseURI, responseHash, tag, timestamp

**ValidationRequested**
- Triggered when a validation is requested
- Stores: agentId, validator, requestHash, timestamp

## GraphQL API

The indexer automatically generates a GraphQL API at `http://localhost:42069/graphql`.

### Example Queries

Get all events for a specific agent:
```graphql
query {
  events(where: { agentId: "42" }) {
    items {
      id
      chainId
      registry
      eventType
      timestamp
    }
  }
}
```

Get reputation events on Base Sepolia:
```graphql
query {
  events(where: { chainId: "84532", registry: "reputation" }) {
    items {
      agentId
      eventType
      score
      timestamp
    }
  }
}
```

Get latest events across all chains:
```graphql
query {
  events(orderBy: "timestamp", orderDirection: "desc", limit: 100) {
    items {
      chainId
      registry
      eventType
      agentId
      timestamp
    }
  }
}
```

Get indexing status:
```graphql
query {
  checkpoints {
    items {
      chainId
      lastBlockNumber
      lastBlockHash
    }
  }
}
```

## REST API Endpoints

In addition to GraphQL, the following REST endpoints are available:

- `GET /health` - Health check endpoint
- `GET /status` - Indexing status with stats
- `POST /graphql` - GraphQL endpoint

## Database Schema

The indexer creates two tables in PostgreSQL:

### Event Table
Stores all indexed blockchain events with the following fields:
- `id` (string, primary key)
- `chainId` (bigint)
- `blockNumber` (bigint)
- `blockHash` (string)
- `transactionHash` (string)
- `logIndex` (int)
- `registry` (string: 'identity' | 'reputation' | 'validation')
- `eventType` (string)
- `agentId` (bigint, optional)
- `timestamp` (bigint)
- Registry-specific fields (owner, score, validator, etc.)

### Checkpoint Table
Tracks the last processed block for each chain:
- `chainId` (bigint, primary key)
- `lastBlockNumber` (bigint)
- `lastBlockHash` (string)

## Development

### Run All Checks (Pre-Commit)
```bash
pnpm check   # Runs tests + lint + typecheck
```

### Run Tests
```bash
pnpm test                    # Run all tests
pnpm test:coverage           # Run with coverage report
```

### Type Check
```bash
pnpm typecheck               # Uses tsconfig.check.json
```

### Lint
```bash
pnpm lint
pnpm lint:fix
```

### Format Code
```bash
pnpm format
pnpm format:check
```

### Generate TypeScript Types
```bash
pnpm codegen
```

### Pre-Commit Hooks

This project uses Husky pre-commit hooks to ensure code quality. Before every commit, the following checks run automatically:

1. Unit tests (`pnpm test`)
2. Linting (`pnpm lint`)
3. Type checking (`pnpm typecheck`)

To bypass hooks (not recommended):
```bash
git commit --no-verify -m "message"
```

## Recent Updates

### November 2025 (Latest): Security Hardening - Complete Validation Suite

Major security hardening with comprehensive validation, structured logging, and test coverage.

**What Changed** (commit `6e1d19c`):

1. **Zod v4 Environment Validation** (`src/env-validation.ts`):
   - Strict schema validation at startup
   - HTTPS-only RPC URL enforcement (security requirement)
   - Type-safe environment access
   - Clear error messages for missing/invalid config

2. **Pino Structured Logging** (`src/logger.ts`):
   - Production-ready structured JSON logs
   - Automatic credential redaction (API keys, passwords in URLs)
   - Separate loggers per component (config, rpc, event)
   - Pretty printing in development mode

3. **Unit Test Suite** (38 tests):
   - Environment validation tests (valid/invalid configs, edge cases)
   - Logger tests (formatting, redaction, levels)
   - Run with `pnpm test` or `pnpm test:coverage`

4. **Pre-Commit Hooks** (Husky):
   - Runs `pnpm check` before every commit
   - Prevents broken code from being committed
   - Validates: tests, lint, typecheck

5. **TypeScript Strict Mode**:
   - Separate `tsconfig.check.json` for standalone typechecking
   - Excludes Ponder codegen files (require `pnpm codegen`)
   - ESLint + Prettier integration

**Dependencies Added**:
- `zod@^4.1.13` - Environment validation
- `pino@^10.1.0` - Structured logging
- `pino-pretty@^13.1.2` - Development pretty printing
- `husky@^9.1.7` - Git hooks

### November 2025: Security Enhancement - Environment-Based Configuration

All contract addresses and start blocks moved to environment variables for enhanced security and flexibility.

**What Changed**:
- Previously: Contract addresses hardcoded in `ponder.config.ts`
- Now: All addresses loaded from `.env` file
- Impact: Secure production deployments, easy environment management

**Details**: See commit `fc7a4fb` and [CONTRACTS.md](./CONTRACTS.md) for configuration instructions.

**Environment Variables Added** (16 total):
- 12 contract addresses (3 registries × 4 networks)
- 4 start block numbers (1 per network)

Example:
```bash
ETHEREUM_SEPOLIA_IDENTITY_ADDRESS=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb
ETHEREUM_SEPOLIA_START_BLOCK=5000000
```

## Configuration

### ponder.config.ts

The main configuration file contains:
- Network configurations (RPC URLs, chain IDs)
- Contract addresses for each registry on each network
- Database connection string
- Start block numbers for each contract

### tsconfig.json

TypeScript configuration with strict mode enabled:
- `strict: true`
- `noImplicitAny: true`
- `noUnusedLocals: true`
- `noUnusedParameters: true`

## Performance Optimization

### Start Block Configuration
For faster initial sync, set the `startBlock` in `ponder.config.ts` to the deployment block of each contract:

```typescript
IdentityRegistryEthereumSepolia: {
  network: "ethereumSepolia",
  abi: IdentityRegistryAbi,
  address: "0x...",
  startBlock: 5000000, // Deployment block
}
```

### RPC Configuration
- Use dedicated RPC providers (Alchemy, Infura) instead of public endpoints
- Consider using multiple RPC URLs for redundancy
- Set appropriate rate limits in Ponder config

### Database Optimization
- Ensure PostgreSQL has sufficient resources
- Use TimescaleDB for better time-series performance
- Monitor query performance with EXPLAIN ANALYZE

## Monitoring

### Logs
Ponder logs indexing progress to stdout. Set log level via environment:
```bash
PONDER_LOG_LEVEL=debug pnpm dev
```

### Status Endpoint
Check indexing status:
```bash
curl http://localhost:42069/status
```

Response:
```json
{
  "status": "healthy",
  "indexer": {
    "totalEvents": 12345,
    "chains": [
      {
        "chainId": "11155111",
        "lastBlockNumber": "5000000",
        "lastBlockHash": "0x..."
      }
    ]
  },
  "timestamp": "2025-01-23T12:00:00.000Z"
}
```

## Troubleshooting

### Ponder won't start
- Ensure PostgreSQL is running: `docker-compose ps`
- Check DATABASE_URL is correct
- Verify RPC URLs are valid and have sufficient quota

### Events not indexing
- Check contract addresses are correct (not 0x0000...)
- Verify ABIs match deployed contracts
- Ensure startBlock is set to deployment block or earlier
- Check RPC provider rate limits

### GraphQL errors
- Verify schema matches database tables
- Check event handler types match ABI events
- Run `pnpm codegen` to regenerate types

### Slow syncing
- Reduce startBlock to deployment block
- Use faster RPC providers
- Enable connection pooling in DATABASE_URL
- Increase RPC timeout: `PONDER_RPC_REQUEST_TIMEOUT=60000`

## Integration with Backend

The Ponder indexer shares the same PostgreSQL database as the Rust backend. Events are automatically available to:

- **Event Processor** - Reads events and evaluates triggers
- **Trigger Engine** - Matches events against user-defined conditions
- **Action Workers** - Executes actions when triggers match

## Adding New Chains

To add support for a new chain:

1. Add network configuration to `ponder.config.ts`:
```typescript
networks: {
  myNewNetwork: {
    chainId: 123456,
    transport: http(process.env.MY_NEW_NETWORK_RPC_URL || ""),
  },
}
```

2. Add contract configurations:
```typescript
contracts: {
  IdentityRegistryMyNewNetwork: {
    network: "myNewNetwork",
    abi: IdentityRegistryAbi,
    address: "0x...",
    startBlock: 0,
  },
}
```

3. Add event handlers in `src/index.ts`:
```typescript
ponder.on("IdentityRegistryMyNewNetwork:AgentRegistered", async ({ event, context }) => {
  await handleAgentRegistered(event, context, 123456n);
});
```

4. Add RPC URL to `.env`:
```bash
MY_NEW_NETWORK_RPC_URL=https://...
```

## Security

- Never commit `.env` files
- Use environment variables for all secrets
- Validate all event data before storing
- Use parameterized queries (Ponder handles this automatically)
- Monitor for unusual event patterns

## Contributing

See the root [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.

## Support

- Documentation: [CLAUDE.md](../CLAUDE.md)
- Issues: [GitHub Issues](https://github.com/matteoscurati/api.8004.dev/issues)
- Ponder Docs: https://ponder.sh

## Related Links

- Ponder Documentation: https://ponder.sh
- Viem Documentation: https://viem.sh
- ERC-8004 Standard: https://eips.ethereum.org/EIPS/eip-8004
