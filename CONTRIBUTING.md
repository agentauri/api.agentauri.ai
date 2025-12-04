# Contributing to api.agentauri.ai

Thank you for your interest in contributing to api.agentauri.ai! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Documentation](#documentation)

## Code of Conduct

This project follows standard open-source etiquette:
- Be respectful and constructive
- Welcome newcomers and help them get started
- Focus on what is best for the community
- Show empathy towards other contributors

## Getting Started

### Prerequisites

- Rust 1.70+ (with cargo, rustfmt, clippy)
- Node.js 20+ with pnpm
- PostgreSQL 15+ with TimescaleDB extension
- Redis 7+
- Docker and Docker Compose

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/api.agentauri.ai.git
   cd api.agentauri.ai
   ```
3. Follow setup instructions in [README.md](./README.md#development)
4. Run initial tests to verify setup:
   ```bash
   ./scripts/local-ci.sh
   ```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

### 2. Make Your Changes

- Write clean, readable code
- Follow the code standards below
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

Run the complete local test suite:

```bash
# Quick check (database + Rust + TypeScript)
./scripts/local-ci.sh

# Code quality (linting, formatting)
./scripts/local-lint.sh

# Security audit
./scripts/local-security.sh

# Or run everything
./scripts/local-all.sh
```

**All tests must pass before submitting a PR.**

### 4. Commit Your Changes

Follow conventional commits format:

```bash
git commit -m "feat: add new trigger condition type"
git commit -m "fix: resolve JWT expiration edge case"
git commit -m "docs: update API documentation for pagination"
git commit -m "test: add integration tests for event processor"
```

Commit types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, semicolons, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

## Code Standards

### Rust

**Location**: `rust-backend/`

**Standards**:
- Follow Rust API Guidelines
- Use `cargo fmt` for formatting (enforced by CI)
- Pass `cargo clippy` with zero warnings (enforced by CI)
- No `unsafe` code without explicit justification
- Comprehensive error handling (no unwrap in production code)
- Add doc comments for public APIs

**Example**:
```rust
/// Creates a new trigger with the given configuration.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - ID of the user creating the trigger
/// * `config` - Trigger configuration
///
/// # Returns
/// Result containing the created trigger ID or an error
pub async fn create_trigger(
    pool: &DbPool,
    user_id: &str,
    config: TriggerConfig,
) -> Result<String> {
    // Implementation
}
```

**File Organization**:
- `crates/shared/`: Shared utilities and configuration
- `crates/api-gateway/`: REST API endpoints
- `crates/event-processor/`: Event processing logic
- `crates/action-workers/`: Action execution (Telegram, REST, MCP)

### TypeScript

**Location**: `ponder-indexers/`

**Standards**:
- Strict TypeScript mode enabled
- Proper type annotations (no `any` unless absolutely necessary)
- Use Ponder-generated types for blockchain data
- ESLint rules followed
- Prettier formatting

**Example**:
```typescript
import { ponder } from "@/generated";

ponder.on("ERC8004Identity:IdentityRegistered", async ({ event, context }) => {
  await context.db.Event.create({
    id: `${event.log.id}-identity-registered`,
    chainId: event.log.chainId,
    blockNumber: event.log.blockNumber,
    // Proper typing for all fields
  });
});
```

### SQL

**Location**: `database/migrations/`

**Standards**:
- Use lowercase for keywords in migrations
- Add comments explaining complex queries
- Include `CREATE INDEX` for foreign keys and common query patterns
- Use `TIMESTAMPTZ` for timestamps (not `TIMESTAMP`)
- Add `CHECK` constraints for enum-like fields

**Example**:
```sql
-- Migration: Add trigger_state table
-- Description: Stores stateful trigger data (EMA, counters, rate limits)

CREATE TABLE trigger_state (
    trigger_id TEXT PRIMARY KEY,
    state_type TEXT NOT NULL CHECK (state_type IN ('ema', 'counter', 'rate_limit')),
    state_data JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_trigger FOREIGN KEY (trigger_id) REFERENCES triggers(id) ON DELETE CASCADE
);

-- Index for common query pattern
CREATE INDEX idx_trigger_state_type ON trigger_state(state_type);
```

## Testing Requirements

### Test Coverage

**All code changes must include tests. Target: 100% coverage.**

### Test Types

1. **Unit Tests**:
   - Rust: `cargo test` in each crate
   - TypeScript: Jest or Vitest

2. **Integration Tests**:
   - Database tests in `database/tests/`
   - API endpoint tests (Week 10)

3. **End-to-End Tests**:
   - Full workflow tests (Week 10)

### Running Tests Locally

```bash
# Database tests (5 test files, 108 tests)
cd database
./test-migrations.sh

# Rust tests
cd rust-backend
cargo test --all

# TypeScript tests
cd ponder-indexers
pnpm test  # (when implemented)

# Or use local test scripts
./scripts/local-ci.sh      # Core tests (2-5 min)
./scripts/local-lint.sh    # Code quality (3-5 min)
./scripts/local-security.sh # Security audit (5-10 min)
```

## Pull Request Process

### Before Submitting

- [ ] All tests pass locally (`./scripts/local-all.sh`)
- [ ] Code follows standards (Rust: clippy, fmt; TypeScript: ESLint, Prettier)
- [ ] Documentation updated (if adding features or changing APIs)
- [ ] Commit messages follow conventional commits
- [ ] Branch is up-to-date with main

### PR Description

Include in your PR description:
1. **What**: Brief description of changes
2. **Why**: Motivation and context
3. **How**: Implementation approach
4. **Testing**: How you tested the changes
5. **Screenshots**: If UI changes (not applicable for this project yet)

### Review Process

1. Automated checks run (CI/CD via GitHub Actions)
2. Code review by maintainer
3. Address feedback if any
4. Approval and merge

### Merge Requirements

- ✅ All CI/CD checks passing
- ✅ At least one approval from maintainer
- ✅ No merge conflicts
- ✅ Branch is up-to-date with main

## Documentation

### When to Update Documentation

Update documentation when:
- Adding new features or APIs
- Changing existing behavior
- Fixing bugs that affect documented behavior
- Adding configuration options

### Documentation Locations

- **User-facing**: `README.md`
- **Technical reference**: `CLAUDE.md`
- **API docs**: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`
- **Architecture**: `docs/architecture/`
- **Development**: `docs/development/`
- **Examples**: `docs/examples/`

### Documentation Standards

- Use clear, concise language
- Include code examples
- Use proper markdown formatting
- Add diagrams where helpful (Mermaid.js or ASCII art)
- Keep docs up-to-date with code changes

## Project Structure

```
api.agentauri.ai/
├── rust-backend/          # Rust microservices
│   ├── crates/shared/     # Shared utilities
│   ├── crates/api-gateway/   # REST API
│   ├── crates/event-processor/ # Event processing
│   └── crates/action-workers/  # Action execution
├── ponder-indexers/       # Blockchain event indexing
├── database/              # PostgreSQL migrations and tests
├── scripts/               # Local testing scripts
├── docs/                  # Documentation
└── .github/workflows/     # CI/CD pipelines
```

## Getting Help

- **Documentation**: See [CLAUDE.md](./CLAUDE.md) for comprehensive technical reference
- **Issues**: Check existing [GitHub Issues](https://github.com/your-org/api.agentauri.ai/issues)
- **Questions**: Open a new issue with the `question` label

## License

By contributing to api.agentauri.ai, you agree that your contributions will be licensed under the MIT License.

---

**Thank you for contributing to api.agentauri.ai!**
