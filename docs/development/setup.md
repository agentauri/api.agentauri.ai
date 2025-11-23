# Development Setup Guide

## Prerequisites

### Required Software

- **Rust** 1.75 or later
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Node.js** 20 or later
  ```bash
  # Using nvm
  nvm install 20
  nvm use 20
  ```

- **pnpm** (Node.js package manager)
  ```bash
  npm install -g pnpm
  ```

- **PostgreSQL** 15 or later with TimescaleDB extension
  ```bash
  # macOS
  brew install postgresql@15 timescaledb

  # Ubuntu/Debian
  sudo apt-get install postgresql-15 postgresql-contrib
  ```

- **Redis** 7 or later
  ```bash
  # macOS
  brew install redis

  # Ubuntu/Debian
  sudo apt-get install redis-server
  ```

- **Docker & Docker Compose** (for local development)
  - Download from: https://www.docker.com/products/docker-desktop

### Optional Tools

- **SQLx CLI** (for database migrations)
  ```bash
  cargo install sqlx-cli --no-default-features --features postgres
  ```

- **k6** (for load testing)
  ```bash
  brew install k6
  ```

## Local Development Setup

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/api.8004.dev.git
cd api.8004.dev
```

### 2. Environment Configuration

Create `.env` file in the project root:

```bash
# Database
DATABASE_URL=postgresql://postgres:password@localhost:5432/erc8004_backend

# Redis
REDIS_URL=redis://localhost:6379

# RPC Providers
ETHEREUM_SEPOLIA_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
BASE_SEPOLIA_RPC_URL=https://base-sepolia.g.alchemy.com/v2/YOUR_API_KEY
LINEA_SEPOLIA_RPC_URL=https://linea-sepolia.infura.io/v3/YOUR_API_KEY
POLYGON_AMOY_RPC_URL=https://polygon-amoy.g.alchemy.com/v2/YOUR_API_KEY

# IPFS
IPFS_GATEWAY_URL=https://gateway.pinata.cloud
PINATA_API_KEY=your_pinata_api_key
PINATA_SECRET_KEY=your_pinata_secret_key

# MCP Bridge
MCP_BRIDGE_URL=http://localhost:3001

# JWT
JWT_SECRET=your_secret_key_change_this_in_production

# API
API_HOST=0.0.0.0
API_PORT=8000

# Log level
RUST_LOG=info,api_gateway=debug,event_processor=debug
```

### 3. Start Infrastructure with Docker Compose

```bash
docker-compose up -d
```

This starts:
- PostgreSQL 15 with TimescaleDB
- Redis 7
- Grafana (for dashboards)
- Prometheus (for metrics)

### 4. Database Setup

Initialize the database:

```bash
# Create database (if not using Docker)
createdb erc8004_backend

# Run migrations
cd rust-backend
sqlx migrate run
```

Verify migrations:

```bash
psql erc8004_backend -c "\dt"
```

### 5. Install Dependencies

**Rust dependencies**:
```bash
cd rust-backend
cargo build
```

**TypeScript dependencies**:
```bash
cd ponder-indexers
pnpm install
```

**MCP Bridge dependencies**:
```bash
cd mcp-bridge-service
pnpm install
```

### 6. Start Services

Open multiple terminal windows/tabs:

**Terminal 1 - Ponder Indexers**:
```bash
cd ponder-indexers
pnpm dev
```

**Terminal 2 - MCP Bridge Service**:
```bash
cd mcp-bridge-service
pnpm dev
```

**Terminal 3 - API Gateway**:
```bash
cd rust-backend/crates/api-gateway
cargo run
```

**Terminal 4 - Event Processor**:
```bash
cd rust-backend/crates/event-processor
cargo run
```

**Terminal 5 - Action Workers**:
```bash
cd rust-backend/crates/action-workers
cargo run
```

### 7. Verify Setup

Check that all services are running:

```bash
# API Gateway health check
curl http://localhost:8000/api/v1/health

# Check PostgreSQL
psql erc8004_backend -c "SELECT COUNT(*) FROM events;"

# Check Redis
redis-cli ping
```

## Development Workflow

### Running Tests

**Rust tests**:
```bash
cd rust-backend
cargo test
```

**TypeScript tests**:
```bash
cd ponder-indexers
pnpm test
```

### Code Formatting

**Rust**:
```bash
cd rust-backend
cargo fmt
```

**TypeScript**:
```bash
cd ponder-indexers
pnpm format
```

### Linting

**Rust**:
```bash
cd rust-backend
cargo clippy
```

**TypeScript**:
```bash
cd ponder-indexers
pnpm lint
```

### Database Migrations

**Create new migration**:
```bash
cd rust-backend
sqlx migrate add <migration_name>
```

**Apply migrations**:
```bash
sqlx migrate run
```

**Revert last migration**:
```bash
sqlx migrate revert
```

## Troubleshooting

### PostgreSQL Connection Issues

If you see "connection refused" errors:

```bash
# Check if PostgreSQL is running
pg_isready

# Start PostgreSQL (macOS)
brew services start postgresql@15

# Start PostgreSQL (Linux)
sudo systemctl start postgresql
```

### TimescaleDB Not Installed

```bash
# macOS
brew install timescaledb
/opt/homebrew/bin/timescaledb-tune

# Ubuntu/Debian
sudo add-apt-repository ppa:timescale/timescaledb-ppa
sudo apt-get update
sudo apt-get install timescaledb-2-postgresql-15
```

Then enable in PostgreSQL:

```sql
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

### Redis Connection Issues

```bash
# Check if Redis is running
redis-cli ping

# Start Redis (macOS)
brew services start redis

# Start Redis (Linux)
sudo systemctl start redis-server
```

### Port Already in Use

If ports 8000, 5432, or 6379 are already in use:

1. Find the process using the port:
   ```bash
   lsof -i :8000
   ```

2. Kill the process or change the port in `.env`

### Ponder Build Errors

Clear Ponder cache:

```bash
cd ponder-indexers
rm -rf .ponder
pnpm dev
```

## Useful Commands

### Database

```bash
# Connect to database
psql erc8004_backend

# Backup database
pg_dump erc8004_backend > backup.sql

# Restore database
psql erc8004_backend < backup.sql

# View table sizes
psql erc8004_backend -c "
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
"
```

### Redis

```bash
# Monitor Redis commands in real-time
redis-cli monitor

# List all keys
redis-cli keys '*'

# Clear all data
redis-cli flushall
```

### Docker

```bash
# View logs
docker-compose logs -f

# Restart services
docker-compose restart

# Stop all services
docker-compose down

# Remove volumes (WARNING: deletes all data)
docker-compose down -v
```

## Next Steps

- Read [CLAUDE.md](../../CLAUDE.md) for comprehensive project documentation
- Review [Database Schema](../database/schema.md)
- Explore [API Documentation](../api/rest-api-spec.md)
- Check out [Example Triggers](../examples/trigger-examples.md)
