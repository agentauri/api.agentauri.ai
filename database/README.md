# Database Migrations and Schema

This directory contains all database migrations, schema documentation, and seed files for the api.8004.dev PostgreSQL database.

## Directory Structure

```
database/
├── migrations/          # SQLx migration files (ordered by timestamp)
├── seeds/              # Test data and seed files
├── schema.sql          # Full schema reference (all migrations combined)
└── README.md           # This file
```

## Prerequisites

- PostgreSQL 15+
- TimescaleDB extension
- SQLx CLI (for running migrations)

## Installation

### 1. Install PostgreSQL and TimescaleDB

**macOS (using Homebrew):**
```bash
brew install postgresql@15
brew install timescaledb

# Follow TimescaleDB setup instructions
timescaledb-tune --quiet --yes
```

**Ubuntu/Debian:**
```bash
sudo apt install postgresql-15
sudo sh -c "echo 'deb https://packagecloud.io/timescale/timescaledb/ubuntu/ $(lsb_release -c -s) main' > /etc/apt/sources.list.d/timescaledb.list"
wget --quiet -O - https://packagecloud.io/timescale/timescaledb/gpgkey | sudo apt-key add -
sudo apt update
sudo apt install timescaledb-2-postgresql-15
sudo timescaledb-tune --quiet --yes
```

### 2. Create Database

```bash
# Connect to PostgreSQL
psql -U postgres

# Create database
CREATE DATABASE erc8004_backend;

# Exit psql
\q
```

### 3. Install SQLx CLI

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

## Running Migrations

### Apply All Migrations

```bash
# Set DATABASE_URL environment variable
export DATABASE_URL="postgresql://postgres:password@localhost:5432/erc8004_backend"

# Run all pending migrations
sqlx migrate run --source database/migrations
```

### Check Migration Status

```bash
sqlx migrate info --source database/migrations
```

### Revert Last Migration

```bash
sqlx migrate revert --source database/migrations
```

## Migration Files

All migrations follow SQLx naming convention: `YYYYMMDDHHMMSS_description.sql`

| Migration | File | Description |
|-----------|------|-------------|
| 1 | `20250123000001_enable_extensions.sql` | Enable TimescaleDB and pgcrypto extensions |
| 2 | `20250123000002_create_helper_functions.sql` | Create `update_updated_at_column()` function |
| 3 | `20250123000003_create_users_table.sql` | Create users table with indexes and triggers |
| 4 | `20250123000004_create_triggers_table.sql` | Create triggers table for event monitoring |
| 5 | `20250123000005_create_trigger_conditions_table.sql` | Create trigger conditions table |
| 6 | `20250123000006_create_trigger_actions_table.sql` | Create trigger actions table |
| 7 | `20250123000007_create_trigger_state_table.sql` | Create trigger state table for stateful triggers |
| 8 | `20250123000008_create_events_table.sql` | Create events table and notification trigger |
| 9 | `20250123000009_convert_events_to_hypertable.sql` | Convert events to TimescaleDB hypertable |
| 10 | `20250123000010_create_checkpoints_table.sql` | Create checkpoints table for indexer sync |
| 11 | `20250123000011_create_action_results_table.sql` | Create action results audit table |
| 12 | `20250123000012_create_agent_mcp_tokens_table.sql` | Create MCP tokens table (optional) |

## Seeding Test Data

Load test data for development:

```bash
psql -U postgres -d erc8004_backend -f database/seeds/test_data.sql
```

Test data includes:
- 3 test users (alice, bob, charlie) with password "password123"
- 3 sample triggers with conditions and actions
- Sample events (reputation, identity, validation)
- Sample action results
- Sample checkpoints and MCP tokens

## Schema Overview

### Core Tables

- **users**: User accounts for API authentication
- **triggers**: User-defined trigger configurations
- **trigger_conditions**: Matching conditions for triggers
- **trigger_actions**: Actions to execute when triggers match
- **trigger_state**: State storage for stateful triggers (EMA, counters)
- **events**: Immutable log of blockchain events (TimescaleDB hypertable)
- **checkpoints**: Last processed block per chain
- **action_results**: Audit trail of action executions
- **agent_mcp_tokens**: Authentication tokens for agent MCP servers

### Key Features

- **Time-Series Optimization**: Events table uses TimescaleDB hypertable with 7-day chunks
- **Real-Time Notifications**: PostgreSQL NOTIFY on new events
- **Automatic Timestamps**: Triggers automatically update `updated_at` columns
- **Referential Integrity**: Foreign keys with appropriate CASCADE rules
- **Performance Indexes**: Optimized indexes for common query patterns

## Development Workflow

### Creating a New Migration

```bash
# Create a new migration file
sqlx migrate add -r your_migration_name --source database/migrations

# Edit the generated files
# .up.sql: Changes to apply
# .down.sql: How to revert changes

# Apply the migration
sqlx migrate run --source database/migrations
```

### Testing Migrations

```bash
# Apply migration
sqlx migrate run --source database/migrations

# Test your changes
# ...

# Revert if needed
sqlx migrate revert --source database/migrations
```

## Backup and Restore

### Backup Database

```bash
# Full backup (custom format, compressed)
pg_dump -Fc erc8004_backend > backup_$(date +%Y%m%d).dump

# Schema only
pg_dump -s erc8004_backend > schema_backup.sql

# Data only
pg_dump -a erc8004_backend > data_backup.sql
```

### Restore Database

```bash
# From custom format dump
pg_restore -d erc8004_backend backup_20250123.dump

# From SQL file
psql -d erc8004_backend -f backup.sql
```

## Monitoring

### Check Database Size

```sql
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### Check Index Usage

```sql
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;
```

### Check Active Connections

```sql
SELECT count(*) FROM pg_stat_activity WHERE datname = 'erc8004_backend';
```

## Troubleshooting

### TimescaleDB Extension Not Found

```bash
# Check if TimescaleDB is installed
psql -U postgres -c "SELECT * FROM pg_available_extensions WHERE name = 'timescaledb';"

# If not found, install TimescaleDB for your PostgreSQL version
```

### Migration Fails: Extension Missing

```bash
# Make sure you're running migrations as a superuser or user with CREATE EXTENSION privilege
# Or create extensions manually first:
psql -U postgres -d erc8004_backend -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"
psql -U postgres -d erc8004_backend -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"
```

### Cannot Convert to Hypertable

Make sure the events table is empty before converting to hypertable, or migration 9 will fail. If you have data, you'll need to migrate it manually.

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/15/)
- [TimescaleDB Documentation](https://docs.timescale.com/)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [Schema Design Documentation](/docs/database/schema.md)
