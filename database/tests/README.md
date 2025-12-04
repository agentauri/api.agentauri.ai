# Database Test Suite

Comprehensive test suite for PostgreSQL + TimescaleDB database setup, achieving 100% coverage of database functionality as required by the project testing policy.

## Overview

This test suite verifies ALL aspects of the database:
- 12 migration files apply successfully and in order
- All 9 tables, columns, indexes, and constraints are created correctly
- TimescaleDB hypertable configuration and functionality
- Data integrity rules (foreign keys, constraints, cascades)
- PostgreSQL NOTIFY/LISTEN for real-time event processing
- Query performance and index usage
- JSONB field operations

## Test Files

### 1. `test-schema.sql`
**Coverage**: Table existence, columns, data types, primary keys, foreign keys, CHECK constraints, indexes, triggers, unique constraints, NOT NULL constraints

**Tests**: 50+ individual tests organized into 9 test groups:
- Table existence (9 tables)
- Column data types and structure
- Primary key definitions
- Foreign key relationships
- CHECK constraints (registry, action_type, status)
- Index definitions and types
- Database triggers and functions
- Unique constraints
- NOT NULL constraints

**Run individually**:
```bash
docker exec agentauri-postgres psql -U postgres -d test_agentauri_backend -f database/tests/test-schema.sql
```

### 2. `test-timescaledb.sql`
**Coverage**: TimescaleDB hypertable configuration, chunk management, partitioning, time-series queries

**Tests**: 20+ tests organized into 6 test groups:
- Hypertable existence and configuration
- Chunk creation and management
- Data insertion across time ranges
- Query performance with partitioning
- Hypertable metadata verification
- Time-series specific queries (time_bucket, aggregations)

**Run individually**:
```bash
docker exec agentauri-postgres psql -U postgres -d test_agentauri_backend -f database/tests/test-timescaledb.sql
```

### 3. `test-data-integrity.sql`
**Coverage**: Foreign key enforcement, CASCADE deletes, CHECK constraints, unique constraints, NOT NULL validation, JSONB fields

**Tests**: 25+ tests organized into 6 test groups:
- Foreign key constraint enforcement
- CASCADE delete behavior
- CHECK constraint validation
- Unique constraint enforcement
- NOT NULL constraint enforcement
- JSONB field validation and querying

**Run individually**:
```bash
docker exec agentauri-postgres psql -U postgres -d test_agentauri_backend -f database/tests/test-data-integrity.sql
```

### 4. `test-notifications.sql`
**Coverage**: PostgreSQL NOTIFY/LISTEN functionality, trigger behavior, notification payloads

**Tests**: 15+ tests organized into 4 test groups:
- NOTIFY function and trigger existence
- Notification payload format
- Trigger behavior (INSERT only, not UPDATE/DELETE)
- Edge cases (NULL fields, long IDs, special characters, rollback)

**Run individually**:
```bash
docker exec agentauri-postgres psql -U postgres -d test_agentauri_backend -f database/tests/test-notifications.sql
```

### 5. `test-performance.sql`
**Coverage**: Index usage verification, query performance benchmarks, partial index effectiveness

**Tests**: 25+ tests organized into 6 test groups:
- Index usage on users table
- Index usage on triggers table (including partial indexes)
- Index usage on events table (TimescaleDB indexes)
- Index usage on action_results table
- Query performance benchmarks (INSERT, SELECT, JOIN, aggregations)
- Partial index effectiveness verification

**Run individually**:
```bash
docker exec agentauri-postgres psql -U postgres -d test_agentauri_backend -f database/tests/test-performance.sql
```

## Test Runner Script

### `test-migrations.sh`

Main test runner script that orchestrates all tests in the correct order.

**Features**:
- Checks Docker container status
- Verifies PostgreSQL and TimescaleDB availability
- Creates isolated test database
- Runs all 12 migrations in order
- Tests migration idempotency
- Executes all test suites
- Seeds test data for performance tests
- Tests rollback scenarios
- Provides detailed pass/fail reporting
- Calculates coverage statistics
- Cleans up after completion

**Usage**:
```bash
# Run all tests
./database/test-migrations.sh

# Keep test database for inspection
KEEP_TEST_DB=1 ./database/test-migrations.sh

# Use custom database name
TEST_DB_NAME=my_test_db ./database/test-migrations.sh
```

**Environment Variables**:
- `TEST_DB_NAME`: Test database name (default: `test_agentauri_backend`)
- `DB_USER`: PostgreSQL user (default: `postgres`)
- `DB_HOST`: PostgreSQL host (default: `localhost`)
- `DB_PORT`: PostgreSQL port (default: `5432`)
- `DOCKER_CONTAINER`: Docker container name (default: `agentauri-postgres`)
- `KEEP_TEST_DB`: Set to `1` to keep test database after tests (default: `0`)

## Quick Start

### Prerequisites
- Docker with agentauri-postgres container running
- PostgreSQL 15+ with TimescaleDB 2.x
- Bash shell (macOS/Linux)

### Run All Tests
```bash
# From project root
./database/test-migrations.sh
```

### Expected Output
```
========================================================================
DATABASE MIGRATION TEST SUITE
========================================================================
[INFO] Test Database: test_agentauri_backend
[INFO] Docker Container: agentauri-postgres

[✓] Docker container is running
[✓] PostgreSQL is accessible
[✓] TimescaleDB extension is available
[✓] Test database created
[✓] All 12 migrations applied successfully
...
========================================================================
TEST SUMMARY
========================================================================
Total Tests:  135
Passed:       135
Failed:       0

Pass Rate:    100%

========================================================================
✓ ALL TESTS PASSED
========================================================================
```

## Test Coverage

### Database Objects Covered
- **Tables**: 9/9 (100%)
- **Columns**: 60+/60+ (100%)
- **Primary Keys**: 9/9 (100%)
- **Foreign Keys**: 5/5 (100%)
- **CHECK Constraints**: 5/5 (100%)
- **Indexes**: 20+/20+ (100%)
- **Triggers**: 5/5 (100%)
- **Functions**: 2/2 (100%)
- **Hypertables**: 1/1 (100%)

### Functionality Covered
- ✅ Migration application and ordering
- ✅ Migration idempotency
- ✅ Table creation and structure
- ✅ Data type validation
- ✅ Constraint enforcement
- ✅ Foreign key cascades
- ✅ Index creation and usage
- ✅ Trigger execution
- ✅ TimescaleDB hypertable conversion
- ✅ Time-series partitioning
- ✅ PostgreSQL NOTIFY/LISTEN
- ✅ JSONB field operations
- ✅ Query performance
- ✅ Transaction rollback behavior

## Adding New Tests

### 1. Create New Test File
```sql
-- database/tests/test-your-feature.sql
\set ON_ERROR_STOP on

CREATE TEMP TABLE IF NOT EXISTS test_results (
    test_name TEXT,
    status TEXT,
    message TEXT
);

CREATE OR REPLACE FUNCTION record_test(test_name TEXT, passed BOOLEAN, message TEXT DEFAULT '')
RETURNS void AS $$
BEGIN
    INSERT INTO test_results VALUES (
        test_name,
        CASE WHEN passed THEN 'PASS' ELSE 'FAIL' END,
        message
    );
END;
$$ LANGUAGE plpgsql;

-- Your tests here
DO $$
BEGIN
    PERFORM record_test(
        'T1.1: Your test description',
        condition_to_check,
        'Expected behavior description'
    );
END $$;

-- Display results at end (copy from existing test files)
```

### 2. Add to Test Runner
Edit `database/test-migrations.sh` and add:
```bash
run_test_file "database/tests/test-your-feature.sql"
```

### 3. Test Naming Convention
- Use format: `TX.Y: Description`
- Where X = test group number, Y = test number in group
- Be descriptive and specific

## Troubleshooting

### Docker Container Not Running
```bash
# Check container status
docker ps -a | grep agentauri-postgres

# Start container
docker-compose up -d postgres
```

### TimescaleDB Extension Not Available
```bash
# Check PostgreSQL logs
docker logs agentauri-postgres

# Verify TimescaleDB is installed
docker exec agentauri-postgres psql -U postgres -c "SELECT * FROM pg_available_extensions WHERE name = 'timescaledb';"
```

### Migration Fails
```bash
# Check which migration failed
./database/test-migrations.sh 2>&1 | grep "Migration failed"

# Run migration manually to see detailed error
docker exec -i agentauri-postgres psql -U postgres -d test_agentauri_backend < database/migrations/FAILED_MIGRATION.sql
```

### Test Fails
```bash
# Keep test database for inspection
KEEP_TEST_DB=1 ./database/test-migrations.sh

# Connect to test database
docker exec -it agentauri-postgres psql -U postgres -d test_agentauri_backend

# Inspect tables, data, and run queries manually
\dt
\d+ table_name
SELECT * FROM test_table;
```

### View Detailed Test Output
```bash
# Run specific test file with verbose output
docker exec -i agentauri-postgres psql -U postgres -d test_agentauri_backend -v ON_ERROR_STOP=1 < database/tests/test-schema.sql
```

## Integration with CI/CD

### GitHub Actions Example
```yaml
name: Database Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: timescale/timescaledb:latest-pg15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3

      - name: Run Database Tests
        env:
          DOCKER_CONTAINER: postgres
          DB_USER: postgres
        run: ./database/test-migrations.sh

      - name: Upload Test Results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: /tmp/test_output.log
```

## Performance Expectations

### Test Execution Times
- Schema validation: < 5 seconds
- TimescaleDB tests: < 10 seconds
- Data integrity tests: < 15 seconds
- Notification tests: < 5 seconds
- Performance tests: < 20 seconds
- **Total suite**: < 60 seconds

### Performance Benchmarks
Tests verify these performance expectations:
- 100 event inserts: < 5 seconds
- 30-day range query: < 1 second
- Aggregation query: < 1 second
- Multi-table JOIN: < 1 second

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/15/)
- [TimescaleDB Documentation](https://docs.timescale.com/)
- [Database Schema Design](../schema.sql)
- [Migration Files](../migrations/)
- [Testing Policy](../../CLAUDE.md#quality-standards--testing-policy)

## Maintenance

### When to Update Tests

Update tests when:
- Adding new migrations
- Modifying existing schema
- Adding new constraints or indexes
- Changing data integrity rules
- Performance requirements change

### Test Review Checklist

Before committing database changes:
- [ ] All existing tests pass
- [ ] New tests added for new functionality
- [ ] Test coverage remains at 100%
- [ ] Performance benchmarks still met
- [ ] Documentation updated
- [ ] No commented-out tests (remove or fix)

---

**Last Updated**: 2025-01-23
**Version**: 1.0.0
**Test Coverage**: 100%
