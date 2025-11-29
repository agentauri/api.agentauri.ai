# Testing Guide for CachedStateManager

## Prerequisites

### 1. PostgreSQL Database

The tests require a running PostgreSQL database. Set the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgresql://postgres:YOUR_PASSWORD@localhost:5432/erc8004_backend"
```

### 2. Redis Server

The caching tests require a running Redis server **without authentication** for testing purposes.

**Option A: Docker Compose (Recommended)**

```bash
# Start Redis without auth
docker-compose up -d redis
```

**Option B: Local Redis**

Configure Redis to allow connections without authentication (for testing only):

```bash
# redis.conf
requirepass ""
protected-mode no
```

Then start Redis:

```bash
redis-server redis.conf
```

Set the Redis URL:

```bash
export REDIS_URL="redis://localhost:6379"
```

### 3. Verify Connectivity

```bash
# Test PostgreSQL
psql $DATABASE_URL -c "SELECT 1;"

# Test Redis
redis-cli ping
# Should return: PONG
```

## Running Tests

### All Tests

```bash
cd rust-backend/crates/event-processor

# Run all tests (requires both PostgreSQL and Redis)
cargo test cached_state_manager
```

### Unit Tests Only

```bash
# Run only unit tests (simpler tests, some may not need Redis)
cargo test --lib cached_state_manager::tests
```

### Integration Tests Only

```bash
# Run integration tests (requires both PostgreSQL and Redis)
cargo test --test cached_state_manager_integration
```

### Specific Test

```bash
# Run a single test
cargo test --lib cached_state_manager::tests::test_cache_key_format -- --nocapture
```

## Benchmarks

Running benchmarks requires both PostgreSQL and Redis to be available:

```bash
cd rust-backend/crates/event-processor
cargo bench --bench state_cache_benchmark
```

Expected runtime: 30-60 seconds

## Troubleshooting

### Redis Authentication Error

**Error**: `NOAUTH: Authentication required`

**Solution**: Redis is configured with authentication. For testing, use a Redis instance without auth:

```bash
# Option 1: Use docker-compose
docker-compose up -d redis

# Option 2: Configure local Redis without auth
# Edit redis.conf and set: requirepass ""
redis-server redis.conf

# Option 3: Skip Redis-dependent tests
cargo test cached_state_manager::tests::test_cache_key_format
cargo test cached_state_manager::tests::test_cache_stats
cargo test cached_state_manager::tests::test_load_state_nonexistent
```

### Database Connection Error

**Error**: `Failed to connect to test database`

**Solution**:

```bash
# Verify DATABASE_URL is set
echo $DATABASE_URL

# Test connection
psql $DATABASE_URL -c "SELECT 1;"

# Start PostgreSQL if not running
docker-compose up -d postgres
```

### Redis Connection Refused

**Error**: `Connection refused`

**Solution**:

```bash
# Check if Redis is running
redis-cli ping

# Start Redis
docker-compose up -d redis

# Or start local Redis
redis-server
```

## Test Coverage

The test suite includes:

- **15 unit tests**: Cache key format, stats, basic operations
- **15 integration tests**: Full stack testing with PostgreSQL + Redis
- **5 benchmark suites**: Performance measurement

### Unit Tests (src/cached_state_manager.rs)

1. `test_cache_key_format` - Verify cache key naming
2. `test_cache_stats` - Verify stats API
3. `test_load_state_cache_miss_then_hit` - Cache miss/hit flow
4. `test_update_state_write_through` - Write-through pattern
5. `test_update_state_overwrites_cache` - Cache invalidation on update
6. `test_delete_state_removes_from_both` - Delete from both stores
7. `test_cache_disabled_fallback` - Fallback when cache disabled
8. `test_load_state_nonexistent` - Non-existent state handling
9. `test_cache_ttl_expiration` - TTL expiration behavior
10. `test_concurrent_updates` - Concurrent write safety
11. `test_cleanup_expired` - Cleanup old state
12. `test_get_state_count` - State counting
13. `test_complex_state_structure` - Complex JSONB handling
14. `test_large_state_data` - Large state handling
15. `test_state_upsert_atomicity` - Atomic UPSERT

### Integration Tests (tests/cached_state_manager_integration.rs)

1. `test_cache_hit_performance` - Measure cache hit latency
2. `test_cache_miss_fallback` - Cache miss → DB fallback
3. `test_write_through_consistency` - Consistency validation
4. `test_redis_failure_graceful_degradation` - Redis failure handling
5. `test_high_throughput_scenario` - High-load testing (1000 events)
6. `test_cache_ttl_behavior` - TTL verification
7. `test_cache_invalidation_on_delete` - Delete invalidation
8. `test_multiple_managers_consistency` - Multi-instance consistency
9. `test_large_state_caching` - Large state (1000 timestamps)
10. `test_cache_stats` - Stats API integration
11. `test_cleanup_expired_with_cache` - Cleanup with caching

### Benchmarks (benches/state_cache_benchmark.rs)

1. `bench_single_read_latency` - Cache vs DB read latency
2. `bench_write_latency` - Write overhead measurement
3. `bench_mixed_workload` - 80/20 read/write ratio
4. `bench_concurrent_access` - 100 triggers, 1000 events
5. `bench_cache_hit_rate` - Measure cache hit rate

## CI/CD

For continuous integration, ensure:

1. PostgreSQL service is running
2. Redis service is running (without auth)
3. Environment variables are set

**GitHub Actions Example**:

```yaml
services:
  postgres:
    image: timescale/timescaledb:latest-pg15
    env:
      POSTGRES_PASSWORD: testpassword
      POSTGRES_DB: erc8004_backend
    ports:
      - 5432:5432

  redis:
    image: redis:7-alpine
    ports:
      - 6379:6379

env:
  DATABASE_URL: postgresql://postgres:testpassword@localhost:5432/erc8004_backend
  REDIS_URL: redis://localhost:6379
```

## Performance Expectations

### Test Execution Times

- Unit tests: ~0.5-1 second
- Integration tests: ~2-5 seconds
- Benchmarks: ~30-60 seconds

### Benchmark Results

Expected performance improvements:

| Metric | Without Cache | With Cache | Improvement |
|--------|---------------|------------|-------------|
| Read latency | 2-5ms | 0.1-0.5ms | 8-100x |
| Throughput (80% read) | 200 ops/sec | 1400 ops/sec | 7x |
| Concurrent (1000 events) | 15s | 2-3s | 5-7x |

## Debugging Tests

### Enable Debug Logging

```bash
RUST_LOG=event_processor=debug cargo test cached_state_manager -- --nocapture
```

### Run Single Test with Backtrace

```bash
RUST_BACKTRACE=1 cargo test --lib cached_state_manager::tests::test_update_state_write_through -- --nocapture
```

### Check Redis State During Tests

```bash
# In another terminal while tests run
redis-cli MONITOR

# Or check keys
redis-cli KEYS "trigger:state:*"
```

## Skipping Tests

If you don't have Redis available, you can skip Redis-dependent tests:

```bash
# Run only tests that don't require Redis
cargo test cached_state_manager::tests::test_cache_key_format
cargo test cached_state_manager::tests::test_load_state_nonexistent
```

Or disable caching in tests:

```bash
STATE_CACHE_ENABLED=false cargo test cached_state_manager
```

This will make all tests fall back to PostgreSQL-only mode.
