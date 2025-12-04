# Integration Test Suite

This directory contains integration tests for the api.agentauri.ai backend infrastructure.

## Test Scripts

### Phase 1: Foundation

- **`test-database-integration.sh`** - Database connectivity and schema tests (7 tests)
- **`test-redis-integration.sh`** - Redis connectivity and operations tests (6 tests)

### Phase 2a: Shadow Mode Rate Limiting

- **`test-rate-limiting-shadow.sh`** - Shadow mode rate limiting tests (14 assertions)
  - Rate limit header validation
  - Shadow mode behavior (allows all requests)
  - Shadow violation detection
  - Anonymous rate limits (10 req/hr)
  - Redis connection health
  - Middleware chain ordering

### Security

- **`test-security-headers.sh`** - Security header validation
- **`test-rate-limiting.sh`** - Basic rate limiting tests

## Running Tests

### Prerequisites

1. **Start Services**:
   ```bash
   docker-compose up -d postgres redis
   ```

2. **Start API Gateway** (for API tests):
   ```bash
   # Shadow mode (Phase 2a)
   RATE_LIMIT_MODE=shadow cargo run --bin api-gateway

   # Enforcing mode (Phase 2b)
   RATE_LIMIT_MODE=enforcing cargo run --bin api-gateway
   ```

### Run Individual Tests

```bash
# Database integration tests
./scripts/tests/test-database-integration.sh

# Redis integration tests
./scripts/tests/test-redis-integration.sh

# Shadow mode rate limiting tests
./scripts/tests/test-rate-limiting-shadow.sh

# All tests
./scripts/tests/run-all-tests.sh
```

### Test Output

Tests use color-coded output:
- 🟢 **GREEN** ([PASS]): Test passed
- 🔴 **RED** ([FAIL]): Test failed
- 🟡 **YELLOW** ([WARN]): Warning (not a failure)
- 🔵 **BLUE** ([INFO]): Information

Example:
```
[TEST] Test 1: Rate limit headers are present on all responses
[PASS] x-ratelimit-limit header present
[PASS] x-ratelimit-remaining header present
[PASS] x-ratelimit-reset header present
[PASS] x-ratelimit-window header present
```

## Test Configuration

### Environment Variables

Tests can be configured with environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `API_BASE_URL` | `http://localhost:8080` | API Gateway URL |
| `DATABASE_URL` | From `.env` | PostgreSQL connection string |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection string |

Example:
```bash
API_BASE_URL=http://localhost:3000 ./scripts/tests/test-rate-limiting-shadow.sh
```

## Test Coverage

### Phase 1 (Foundation)
- ✅ Database connectivity (7/7 tests)
- ✅ Redis connectivity (6/6 tests)
- ✅ Security headers (5/5 tests)

### Phase 2a (Shadow Mode)
- ✅ Rate limiting middleware (14/14 assertions)
- ✅ Shadow mode behavior
- ✅ Response headers (RFC 6585 compliant)

### Phase 2b (Enforcing Mode)
- ⏳ 429 error responses (planned)
- ⏳ Fallback limiter activation (planned)
- ⏳ Rate limit bypass prevention (planned)

### Phase 3 (OAuth 2.0)
- ⏳ OAuth token generation (planned)
- ⏳ OAuth authentication flow (planned)
- ⏳ Token expiration and refresh (planned)

## Continuous Integration

These tests are run automatically in GitHub Actions:

```yaml
# .github/workflows/ci.yml
- name: Run Integration Tests
  run: |
    docker-compose up -d
    ./scripts/tests/run-all-tests.sh
```

## Writing New Tests

### Test Script Template

```bash
#!/bin/bash
# Integration test for <feature>

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Counters
TESTS_PASSED=0
TESTS_FAILED=0

# Print functions
print_test() { echo -e "${BLUE}[TEST]${NC} $1"; }
print_success() { echo -e "${GREEN}[PASS]${NC} $1"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
print_fail() { echo -e "${RED}[FAIL]${NC} $1"; TESTS_FAILED=$((TESTS_FAILED + 1)); }

# Test function
test_something() {
    print_test "Test description"

    # Test logic
    if [ condition ]; then
        print_success "Test passed"
    else
        print_fail "Test failed"
        return 1
    fi
}

# Main
main() {
    echo "========================================" echo "Test Suite Name"
    echo "========================================"

    test_something

    # Summary
    echo "========================================"
    echo "Passed: $TESTS_PASSED"
    echo "Failed: $TESTS_FAILED"
    echo "========================================"

    [ $TESTS_FAILED -eq 0 ]
}

main
exit $?
```

### Best Practices

1. **Use `set -e`**: Exit on first error
2. **Color-coded output**: Use print functions for consistency
3. **Clear test names**: Describe what is being tested
4. **Error messages**: Provide helpful troubleshooting hints
5. **Cleanup**: Reset state between tests if needed
6. **Timeouts**: Add reasonable timeouts for network requests
7. **Documentation**: Add comments explaining complex logic

## Troubleshooting

### Test Failures

**Database tests failing**:
```bash
# Check if PostgreSQL is running
docker-compose ps postgres

# Check database health
psql $DATABASE_URL -c "SELECT 1"

# Review migrations
ls database/migrations/
```

**Redis tests failing**:
```bash
# Check if Redis is running
docker-compose ps redis

# Test Redis connection
redis-cli -h localhost ping

# Check Redis logs
docker-compose logs redis
```

**API tests failing**:
```bash
# Check if API Gateway is running
curl http://localhost:8080/api/v1/health

# Check API Gateway logs
tail -f /tmp/api-gateway-shadow-mode.log

# Restart API Gateway
pkill api-gateway
RATE_LIMIT_MODE=shadow cargo run --bin api-gateway
```

### Common Issues

1. **Port already in use**: Kill existing processes
   ```bash
   lsof -ti:8080 | xargs kill -9
   ```

2. **Redis connection refused**: Start Redis
   ```bash
   docker-compose up -d redis
   ```

3. **Database connection error**: Check DATABASE_URL
   ```bash
   echo $DATABASE_URL
   ```

4. **Rate limit tests flaky**: Clear Redis keys
   ```bash
   redis-cli FLUSHDB
   ```

## Test Maintenance

### When to Update Tests

- **After adding features**: Create tests for new functionality
- **After bug fixes**: Add regression tests
- **After API changes**: Update integration tests
- **After schema changes**: Update database tests

### Test Hygiene

- Run tests before committing: `./scripts/tests/run-all-tests.sh`
- Keep tests fast: Use timeouts, avoid sleep when possible
- Keep tests isolated: Don't depend on external state
- Keep tests maintainable: Use helper functions, clear naming

## Resources

- [Test Script Examples](test-rate-limiting-shadow.sh)
- [API Documentation](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)
- [Phase 2a Documentation](../../docs/phase2a-rate-limiting.md)
- [CLAUDE.md](../../CLAUDE.md)

## License

Copyright (c) 2025 api.agentauri.ai
