# Testing Strategy

**Project**: api.8004.dev - ERC-8004 Backend Infrastructure
**Last Updated**: January 30, 2025
**Audience**: Developers, QA Engineers, CI/CD Maintainers

This document defines the comprehensive testing strategy for the ERC-8004 backend infrastructure, ensuring code quality, reliability, and maintainability.

---

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Pyramid](#test-pyramid)
3. [Coverage Requirements](#coverage-requirements)
4. [Test Types](#test-types)
5. [Testing Tools](#testing-tools)
6. [Test Organization](#test-organization)
7. [Test Data Management](#test-data-management)
8. [Continuous Testing](#continuous-testing)
9. [Best Practices](#best-practices)
10. [Common Patterns](#common-patterns)

---

## Testing Philosophy

### Core Principles

> **"If it's not tested, it's broken. If it's not automatically tested, it will break."**

Our testing philosophy is built on three pillars:

1. **Test-First Development**: Write tests before or alongside implementation
2. **Fast Feedback**: Tests must run quickly (unit tests <1s, integration tests <10s)
3. **Confidence Over Coverage**: 100% coverage of critical paths, pragmatic coverage elsewhere

### Testing Goals

- **Prevent Regressions**: Catch breaking changes before they reach production
- **Enable Refactoring**: Safely improve code structure without fear
- **Document Behavior**: Tests serve as executable documentation
- **Design Validation**: Writing tests improves API design

### Non-Negotiable Rules

✅ **DO**:
- Write tests for all new features
- Run tests before every commit
- Fix failing tests immediately
- Keep tests fast and focused
- Use descriptive test names

❌ **DON'T**:
- Commit code without tests
- Skip failing tests
- Write flaky tests
- Use production data in tests
- Test implementation details

---

## Test Pyramid

Our test distribution follows the testing pyramid pattern:

```
        /\
       /E2E\         10% - End-to-End (slow, high value)
      /------\
     /  INT   \      30% - Integration (medium speed, good coverage)
    /----------\
   /    UNIT    \    60% - Unit (fast, detailed coverage)
  /--------------\
```

### Unit Tests (60%)

**Purpose**: Test individual functions/modules in isolation

**Characteristics**:
- Very fast (<1s per suite)
- No external dependencies (database, network, filesystem)
- Use mocking for dependencies
- High volume (hundreds/thousands of tests)

**Coverage Target**: 80-90% of business logic

**Example** (Rust):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_score_valid() {
        let result = parse_score("75");
        assert_eq!(result.unwrap(), 75);
    }

    #[test]
    fn test_parse_score_out_of_range() {
        let result = parse_score("150");
        assert!(result.is_err());
    }
}
```

### Integration Tests (30%)

**Purpose**: Test component interactions with real dependencies

**Characteristics**:
- Medium speed (1-10s per test)
- Real database/Redis connections
- Test multiple components together
- Moderate volume (tens/hundreds of tests)

**Coverage Target**: 100% of critical workflows

**Example** (Rust):
```rust
#[tokio::test]
#[ignore] // Requires DATABASE_URL
async fn test_trigger_creation_workflow() {
    let pool = setup_test_db().await;

    // Create trigger via API
    let trigger = create_trigger(&pool, "Test Trigger").await.unwrap();

    // Verify in database
    let stored = fetch_trigger(&pool, &trigger.id).await.unwrap();
    assert_eq!(stored.name, "Test Trigger");

    cleanup_test_db(&pool).await;
}
```

### End-to-End Tests (10%)

**Purpose**: Test complete user workflows across entire system

**Characteristics**:
- Slow (10-60s per test)
- Full system integration (all services running)
- Black-box testing through public APIs
- Low volume (10-20 tests)

**Coverage Target**: Critical user journeys only

**Example** (conceptual):
```bash
# E2E test: Blockchain event triggers notification
1. Deploy contracts to testnet
2. Submit NewFeedback transaction
3. Wait for Ponder indexer to process
4. Verify event in database
5. Verify trigger matched
6. Verify Telegram notification sent
```

---

## Coverage Requirements

### Overall Coverage Targets

| Component | Unit Tests | Integration Tests | E2E Tests |
|-----------|-----------|-------------------|-----------|
| **API Gateway** | 80% | 100% endpoints | Critical flows |
| **Event Processor** | 80% | 100% workflows | Event → Action |
| **Action Workers** | 80% | 100% action types | Worker execution |
| **Shared Libraries** | 90% | N/A | N/A |
| **Ponder Indexers** | 70% | 100% handlers | Chain → DB |

### Critical Path Coverage: 100%

**Definition**: Code paths that, if broken, would cause:
- Data loss
- Security vulnerabilities
- System downtime
- Financial loss

**Critical Paths in Our System**:
- Event idempotency (processed_events check)
- Trigger matching logic
- Action job enqueueing
- Circuit breaker state management
- API authentication/authorization
- Database migrations
- Rate limiting enforcement

### Acceptable Gaps

**80% coverage is acceptable for**:
- Logging code
- Error message formatting
- Metrics instrumentation
- CLI argument parsing

**Not tested** (acceptable):
- Main function boilerplate
- Configuration loading (tested indirectly)
- Third-party library wrappers (if thin)

---

## Test Types

### 1. Unit Tests

**Location**:
- Rust: `src/` directory (inline with code)
- TypeScript: `src/__tests__/` or `.test.ts` files

**Naming Convention**:
```rust
// Rust
#[test]
fn test_<functionality>_<scenario>_<expected_outcome>()

// Example
#[test]
fn test_score_threshold_below_60_returns_true()
```

**What to Test**:
- Pure functions (input → output)
- Business logic
- Data transformations
- Validation rules
- Error handling paths

**What NOT to Test**:
- Framework internals
- Database queries (use integration tests)
- Network calls (use integration tests)

### 2. Integration Tests

**Location**:
- Rust: `tests/` directory at crate root
- Database: `database/tests/`

**Setup Requirements**:
- `DATABASE_URL` environment variable
- Test database with migrations applied
- Redis instance (for queue tests)

**Test Template**:
```rust
#[tokio::test]
#[ignore] // Requires DATABASE_URL
async fn test_integration_scenario() {
    // 1. Setup: Create test database state
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    // 2. Execute: Run the workflow
    let result = execute_workflow(&pool).await;

    // 3. Assert: Verify outcome
    assert!(result.is_ok());

    // 4. Cleanup: Remove test data
    cleanup_test_db(&pool).await;
}
```

**Best Practices**:
- Use transactions for isolation (when possible)
- Clean up test data after each test
- Use unique IDs (UUID, timestamp suffix) to avoid conflicts
- Test both success and failure scenarios

### 3. Database Tests

**Location**: `database/tests/`

**Test Categories**:

#### Schema Tests
```sql
-- Test: Foreign key constraints work
BEGIN;
-- Try to insert invalid foreign key
INSERT INTO triggers (organization_id, ...) VALUES ('non_existent_org', ...);
-- Expect: ERROR: violates foreign key constraint
ROLLBACK;
```

#### Migration Tests
```bash
# Test: Migrations apply cleanly
sqlx migrate run
sqlx migrate revert
sqlx migrate run
# All should succeed without errors
```

#### Performance Tests
```sql
-- Test: Query uses index
EXPLAIN ANALYZE
SELECT * FROM triggers
WHERE organization_id = 'org_123' AND enabled = true;
-- Verify: "Index Scan" not "Seq Scan"
```

#### Data Integrity Tests
```sql
-- Test: Trigger delete cascades to conditions
BEGIN;
INSERT INTO triggers (...) VALUES (...) RETURNING id;
INSERT INTO trigger_conditions (trigger_id, ...) VALUES (...);
DELETE FROM triggers WHERE id = ...;
SELECT COUNT(*) FROM trigger_conditions WHERE trigger_id = ...;
-- Expect: 0 (cascade delete worked)
ROLLBACK;
```

### 4. API Tests

**Tool**: `curl`, `httpie`, or test framework

**Test Structure**:
```rust
#[actix_web::test]
async fn test_api_endpoint() {
    let app = test::init_service(App::new().configure(configure_routes)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/triggers")
        .set_json(json!({"name": "Test"}))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: TriggerResponse = test::read_body_json(resp).await;
    assert_eq!(body.name, "Test");
}
```

**What to Test**:
- Success responses (200, 201, 204)
- Error responses (400, 401, 403, 404, 500)
- Authentication/authorization
- Input validation
- Pagination
- Rate limiting

### 5. Performance Tests

**Tools**:
- `criterion` (Rust benchmarking)
- `k6` or `locust` (load testing)

**When to Use**:
- New algorithms (ensure O(n) not O(n²))
- Database query optimization
- API endpoint performance regression
- Capacity planning

**Example** (Criterion):
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_trigger_matching(c: &mut Criterion) {
    c.bench_function("match_100_triggers", |b| {
        b.iter(|| {
            match_triggers(black_box(&event), black_box(&triggers))
        })
    });
}

criterion_group!(benches, benchmark_trigger_matching);
criterion_main!(benches);
```

### 6. Security Tests

**Test Categories**:

#### Authentication Tests
```rust
#[actix_web::test]
async fn test_api_requires_authentication() {
    let req = test::TestRequest::get()
        .uri("/api/v1/triggers")
        .to_request(); // No Authorization header

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}
```

#### Authorization Tests
```rust
#[actix_web::test]
async fn test_user_cannot_access_other_user_triggers() {
    let jwt = create_jwt_for_user("user1");

    let req = test::TestRequest::get()
        .uri("/api/v1/triggers/trigger_owned_by_user2")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}
```

#### Input Validation Tests
```rust
#[actix_web::test]
async fn test_sql_injection_prevention() {
    let malicious_input = "'; DROP TABLE users; --";

    let req = test::TestRequest::post()
        .uri("/api/v1/triggers")
        .set_json(json!({"name": malicious_input}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should either accept as string or reject, but never execute SQL
    assert!(resp.status() == 201 || resp.status() == 400);

    // Verify table still exists
    let users = sqlx::query!("SELECT COUNT(*) FROM users").fetch_one(&pool).await;
    assert!(users.is_ok());
}
```

---

## Testing Tools

### Rust Testing Stack

| Tool | Purpose | Usage |
|------|---------|-------|
| **cargo test** | Test runner | `cargo test` |
| **tokio::test** | Async test runtime | `#[tokio::test]` |
| **mockall** | Mocking framework | Mock traits/structs |
| **criterion** | Benchmarking | Performance regression |
| **proptest** | Property-based testing | Fuzz testing |
| **insta** | Snapshot testing | Large output verification |

### TypeScript Testing Stack

| Tool | Purpose | Usage |
|------|---------|-------|
| **Vitest** | Test runner | `pnpm test` |
| **@testing-library** | UI component testing | React/Vue testing |
| **msw** | API mocking | Mock HTTP requests |
| **playwright** | E2E testing | Browser automation |

### Database Testing Tools

| Tool | Purpose | Usage |
|------|---------|-------|
| **SQLx** | Compile-time SQL verification | `cargo sqlx prepare` |
| **pgTAP** | PostgreSQL unit testing | SQL-based tests |
| **psql** | Manual testing | Interactive queries |

### CI/CD Testing Scripts

Located in `scripts/`:

| Script | Purpose | Runtime |
|--------|---------|---------|
| **local-ci.sh** | Daily development checks | 2-5 min |
| **local-lint.sh** | Pre-PR quality checks | 3-5 min |
| **local-security.sh** | Security audit | 5-10 min |
| **local-all.sh** | Complete validation | 10-15 min |
| **pre-push-check.sh** | Git pre-push hook | 2-3 min |

---

## Test Organization

### Directory Structure

```
api.8004.dev/
├── rust-backend/
│   ├── crates/
│   │   ├── api-gateway/
│   │   │   ├── src/
│   │   │   │   ├── handlers/
│   │   │   │   │   ├── triggers.rs          # Unit tests inline
│   │   │   │   │   └── tests.rs             # Handler tests
│   │   │   │   └── models/
│   │   │   │       └── triggers.rs          # Unit tests inline
│   │   │   └── tests/
│   │   │       ├── api_integration_test.rs  # API integration tests
│   │   │       └── auth_test.rs             # Auth integration tests
│   │   │
│   │   ├── event-processor/
│   │   │   ├── src/
│   │   │   │   ├── processor.rs             # Unit tests inline
│   │   │   │   └── listener.rs              # Unit tests inline
│   │   │   └── tests/
│   │   │       ├── integration_test.rs      # Full workflow tests
│   │   │       └── error_handling_test.rs   # Error scenarios
│   │   │
│   │   └── shared/
│   │       └── src/
│   │           └── models.rs                # Unit tests inline
│   │
│   └── tests/
│       └── e2e_test.rs                      # End-to-end tests
│
├── ponder-indexers/
│   ├── src/
│   │   ├── index.ts
│   │   └── __tests__/
│   │       └── handlers.test.ts             # Handler tests
│   └── test/
│       └── integration.test.ts              # Integration tests
│
└── database/
    └── tests/
        ├── test-schema.sql                  # Schema tests
        ├── test-migrations.sh               # Migration tests
        └── test-performance.sql             # Performance tests
```

### Test Naming Conventions

**Rust Test Files**:
- `tests/<feature>_test.rs` - Integration tests
- `tests/<feature>_integration_test.rs` - Integration tests (explicit)
- `src/<module>.rs` - Unit tests inline with `#[cfg(test)]`

**TypeScript Test Files**:
- `src/__tests__/<feature>.test.ts` - Unit tests
- `test/<feature>.integration.test.ts` - Integration tests

**Test Function Names**:
```rust
// Pattern: test_<what>_<when>_<then>
#[test]
fn test_trigger_matching_score_below_threshold_returns_true()

#[test]
fn test_api_key_validation_invalid_prefix_returns_error()

#[tokio::test]
async fn test_event_processing_duplicate_event_skips_processing()
```

---

## Test Data Management

### Test Database Setup

**Approach**: Isolated test database with migrations

```rust
// Setup helper
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url).await
        .expect("Failed to connect to test database");

    // Clean up previous test data
    cleanup_test_db(&pool).await;

    pool
}

// Cleanup helper
async fn cleanup_test_db(pool: &PgPool) {
    sqlx::query!("DELETE FROM processed_events WHERE event_id LIKE 'test_%'")
        .execute(pool).await.ok();
    sqlx::query!("DELETE FROM events WHERE id LIKE 'test_%'")
        .execute(pool).await.ok();
    // ... more cleanup
}
```

### Test Data Patterns

#### Pattern 1: Use Test-Specific IDs

```rust
// GOOD: Unique test IDs
let event_id = "test_trigger_matching_001";
let trigger_id = "test_trigger_001";

// BAD: Generic IDs (conflicts with other tests)
let event_id = "event1";
let trigger_id = "trigger1";
```

#### Pattern 2: Use Transactions for Isolation

```rust
#[tokio::test]
async fn test_with_transaction() {
    let pool = setup_test_db().await;
    let mut tx = pool.begin().await.unwrap();

    // All operations use &mut tx
    sqlx::query!("INSERT INTO ...").execute(&mut tx).await.unwrap();

    // Automatic rollback on test end (no cleanup needed)
    tx.rollback().await.unwrap();
}
```

#### Pattern 3: Seed Reusable Test Data

```rust
// database/seeds/test_data.sql
INSERT INTO users (id, username, email, password_hash)
VALUES ('test_user_1', 'testuser', 'test@example.com', '$argon2...');

INSERT INTO organizations (id, name, slug, owner_id)
VALUES ('test_org_1', 'Test Org', 'test-org', 'test_user_1');
```

```rust
// In tests
async fn seed_test_user_and_org(pool: &PgPool) {
    let sql = include_str!("../../database/seeds/test_data.sql");
    sqlx::raw_sql(sql).execute(pool).await.unwrap();
}
```

### Test Fixtures

**Option 1: Builder Pattern**

```rust
struct TriggerBuilder {
    name: String,
    chain_id: i32,
    registry: String,
    // ... more fields
}

impl TriggerBuilder {
    fn new() -> Self {
        Self {
            name: "Test Trigger".to_string(),
            chain_id: 84532,
            registry: "reputation".to_string(),
        }
    }

    fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    async fn create(self, pool: &PgPool) -> Trigger {
        // Insert and return trigger
    }
}

// Usage in tests
let trigger = TriggerBuilder::new()
    .name("Custom Trigger")
    .chain_id(11155111)
    .create(&pool)
    .await;
```

**Option 2: Factory Functions**

```rust
async fn create_test_trigger(
    pool: &PgPool,
    name: &str,
    organization_id: &str,
) -> Trigger {
    sqlx::query_as!(
        Trigger,
        "INSERT INTO triggers (id, name, organization_id, ...)
         VALUES ($1, $2, $3, ...) RETURNING *",
        format!("test_trigger_{}", uuid::Uuid::new_v4()),
        name,
        organization_id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}
```

---

## Continuous Testing

### Local Development

**Pre-Commit Hook** (`.git/hooks/pre-commit`):
```bash
#!/bin/bash
# Run fast tests before allowing commit
cargo test --workspace --lib
exit $?
```

**Pre-Push Hook** (`.git/hooks/pre-push`):
```bash
#!/bin/bash
# Run comprehensive checks before push
./scripts/pre-push-check.sh
exit $?
```

### CI/CD Pipeline

**GitHub Actions Workflow** (`.github/workflows/test.yml`):

```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: timescale/timescaledb:latest-pg15
        env:
          POSTGRES_PASSWORD: testpass
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run migrations
        run: |
          cd rust-backend
          sqlx migrate run
        env:
          DATABASE_URL: postgres://postgres:testpass@localhost/erc8004_backend

      - name: Run Rust tests
        run: cargo test --workspace
        env:
          DATABASE_URL: postgres://postgres:testpass@localhost/erc8004_backend
          REDIS_URL: redis://localhost:6379

      - name: Run TypeScript tests
        run: |
          cd ponder-indexers
          pnpm install
          pnpm test
```

### Test Execution Strategy

**Development** (fast feedback):
```bash
# Run only changed tests (watch mode)
cargo watch -x test

# TypeScript watch mode
cd ponder-indexers && pnpm test:watch
```

**Pre-Commit** (very fast):
```bash
# Unit tests only (no database)
cargo test --workspace --lib
```

**Pre-Push** (fast):
```bash
# All tests excluding integration
./scripts/pre-push-check.sh
```

**CI** (comprehensive):
```bash
# All tests including integration and E2E
./scripts/local-all.sh
```

---

## Best Practices

### 1. Test Independence

✅ **DO**: Make tests independent
```rust
#[tokio::test]
async fn test_a() {
    let pool = setup_test_db().await;
    // Create own test data
    let trigger = create_test_trigger(&pool).await;
    // ... test logic
    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_b() {
    let pool = setup_test_db().await;
    // Create own test data (not relying on test_a)
    let trigger = create_test_trigger(&pool).await;
    // ... test logic
    cleanup_test_db(&pool).await;
}
```

❌ **DON'T**: Share state between tests
```rust
// BAD: test_b depends on test_a running first
static SHARED_TRIGGER_ID: &str = "shared_trigger";

#[tokio::test]
async fn test_a() {
    create_trigger_with_id(SHARED_TRIGGER_ID).await;
}

#[tokio::test]
async fn test_b() {
    // Assumes test_a already created trigger
    let trigger = fetch_trigger(SHARED_TRIGGER_ID).await;
}
```

### 2. Descriptive Test Names

✅ **DO**: Use descriptive names
```rust
#[test]
fn test_score_threshold_evaluator_score_below_threshold_returns_true()

#[test]
fn test_api_key_authentication_expired_key_returns_401()

#[test]
fn test_event_processing_duplicate_event_id_skips_and_logs_warning()
```

❌ **DON'T**: Use vague names
```rust
#[test]
fn test_1()

#[test]
fn test_score()

#[test]
fn test_works()
```

### 3. Arrange-Act-Assert Pattern

✅ **DO**: Follow AAA pattern
```rust
#[tokio::test]
async fn test_trigger_creation() {
    // ARRANGE: Setup test data
    let pool = setup_test_db().await;
    let org_id = create_test_organization(&pool).await;

    // ACT: Execute the operation
    let result = create_trigger(&pool, "Test", &org_id).await;

    // ASSERT: Verify the outcome
    assert!(result.is_ok());
    let trigger = result.unwrap();
    assert_eq!(trigger.name, "Test");

    // CLEANUP
    cleanup_test_db(&pool).await;
}
```

### 4. One Assertion Per Test (When Possible)

✅ **DO**: Focus on one behavior
```rust
#[test]
fn test_score_parsing_valid_input_returns_correct_value() {
    let result = parse_score("75");
    assert_eq!(result.unwrap(), 75);
}

#[test]
fn test_score_parsing_out_of_range_returns_error() {
    let result = parse_score("150");
    assert!(result.is_err());
}

#[test]
fn test_score_parsing_negative_returns_error() {
    let result = parse_score("-10");
    assert!(result.is_err());
}
```

❌ **DON'T**: Test multiple behaviors
```rust
#[test]
fn test_score_parsing() {
    assert_eq!(parse_score("75").unwrap(), 75);
    assert!(parse_score("150").is_err());
    assert!(parse_score("-10").is_err());
    assert!(parse_score("abc").is_err());
    // Too many assertions - hard to debug failures
}
```

### 5. Test Error Cases

✅ **DO**: Test both success and failure
```rust
mod trigger_creation {
    #[tokio::test]
    async fn test_valid_trigger_creation_succeeds() {
        // ... happy path
    }

    #[tokio::test]
    async fn test_trigger_creation_invalid_chain_id_returns_error() {
        // ... error case 1
    }

    #[tokio::test]
    async fn test_trigger_creation_duplicate_name_returns_conflict() {
        // ... error case 2
    }

    #[tokio::test]
    async fn test_trigger_creation_nonexistent_organization_returns_error() {
        // ... error case 3
    }
}
```

### 6. Mock External Dependencies

✅ **DO**: Mock external services
```rust
use mockall::predicate::*;
use mockall::mock;

mock! {
    TelegramClient {
        async fn send_message(&self, chat_id: i64, text: &str) -> Result<()>;
    }
}

#[tokio::test]
async fn test_telegram_notification_sends_message() {
    let mut mock_client = MockTelegramClient::new();
    mock_client
        .expect_send_message()
        .with(eq(12345), eq("Test message"))
        .times(1)
        .returning(|_, _| Ok(()));

    let result = send_notification(&mock_client, 12345, "Test message").await;
    assert!(result.is_ok());
}
```

### 7. Avoid Test Interdependence

✅ **DO**: Each test is self-contained
```rust
#[tokio::test]
async fn test_a() {
    let pool = setup_test_db().await;
    // ... independent test
}

#[tokio::test]
async fn test_b() {
    let pool = setup_test_db().await;
    // ... independent test
}
```

❌ **DON'T**: Tests depend on execution order
```rust
// BAD: test_b depends on test_a
#[tokio::test]
async fn test_a_create_user() {
    create_user("test_user").await;
}

#[tokio::test]
async fn test_b_update_user() {
    // Assumes test_a ran first
    update_user("test_user", "new_name").await;
}
```

---

## Common Patterns

### Pattern 1: Testing Async Functions

```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Pattern 2: Testing Error Propagation

```rust
#[tokio::test]
async fn test_error_context_includes_event_id() {
    let result = process_event("invalid_event_id", &pool).await;

    assert!(result.is_err());
    let error_message = format!("{:?}", result.unwrap_err());
    assert!(error_message.contains("invalid_event_id"));
}
```

### Pattern 3: Testing Database Constraints

```rust
#[tokio::test]
async fn test_foreign_key_constraint_prevents_invalid_organization() {
    let pool = setup_test_db().await;

    let result = sqlx::query!(
        "INSERT INTO triggers (organization_id, name, ...) VALUES ($1, $2, ...)",
        "non_existent_org",
        "Test Trigger"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("foreign key"));
}
```

### Pattern 4: Testing Rate Limiting

```rust
#[tokio::test]
async fn test_rate_limit_exceeded_returns_429() {
    let app = setup_test_app().await;

    // Make 101 requests (limit is 100/hour)
    for _ in 0..101 {
        let req = test::TestRequest::get()
            .uri("/api/v1/triggers")
            .insert_header(("X-Forwarded-For", "192.168.1.1"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        if i >= 100 {
            assert_eq!(resp.status(), 429);
        }
    }
}
```

### Pattern 5: Testing Idempotency

```rust
#[tokio::test]
async fn test_event_processing_is_idempotent() {
    let pool = setup_test_db().await;
    let event_id = "test_idempotent_event";

    create_test_event(&pool, event_id).await;

    // Process same event twice
    process_event(event_id, &pool).await.unwrap();
    process_event(event_id, &pool).await.unwrap();

    // Verify processed exactly once
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM processed_events WHERE event_id = $1",
        event_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1);
}
```

---

## Related Documentation

- **Contributing Guide**: [../../CONTRIBUTING.md](../../CONTRIBUTING.md)
- **CI/CD Pipeline**: [../../.github/workflows/README.md](../../.github/workflows/README.md)
- **Database Tests**: [../../database/tests/README.md](../../database/tests/README.md)
- **API Documentation**: [../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)

---

**Last Updated**: January 30, 2025
**Maintainer**: Development Team
**Version**: 1.0.0
