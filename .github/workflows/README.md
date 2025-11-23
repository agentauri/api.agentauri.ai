# GitHub Actions Workflows

This directory contains the CI/CD workflows for the api.8004.dev project.

## Workflows Overview

### 1. CI Workflow (`ci.yml`)

**Triggers:**
- Every push to any branch
- Pull requests to `main` or `develop` branches

**Jobs:**
- **Database Tests**: Runs all 108 database tests using PostgreSQL + TimescaleDB
  - Schema validation
  - TimescaleDB functionality
  - Data integrity constraints
  - PostgreSQL NOTIFY/LISTEN
  - Query performance

- **Rust Tests** (when rust-backend/ exists):
  - Code formatting check (`cargo fmt`)
  - Linting with Clippy
  - Build verification
  - Unit and integration tests

- **TypeScript Tests** (when ponder-indexers/ exists):
  - Type checking
  - ESLint validation
  - Test suite execution

- **Integration Tests**:
  - Runs the master test suite
  - Validates end-to-end functionality

**Test Policy**: All tests must pass before merging. This enforces the 100% test coverage requirement defined in CLAUDE.md.

### 2. Security Scanning Workflow (`security.yml`)

**Triggers:**
- Pull requests to `main` or `develop`
- Weekly on Mondays at 00:00 UTC
- Manual trigger via workflow_dispatch

**Jobs:**
- **Dependency Scan**:
  - NPM audit for TypeScript dependencies
  - Cargo audit for Rust dependencies
  - GitHub dependency review

- **Docker Security Scan**:
  - Trivy vulnerability scanning for all Docker images
  - Results uploaded to GitHub Security tab

- **Dockerfile Linting**:
  - Hadolint checks for Dockerfile best practices

- **SQL Security Analysis**:
  - Checks for SQL injection patterns
  - Scans for hardcoded credentials

- **Secrets Scanning**:
  - Gitleaks scan for committed secrets
  - Prevents credential leaks

- **Code Quality**:
  - Verifies .env is not committed
  - Checks for .env.example
  - Counts TODO/FIXME comments

### 3. Linting Workflow (`lint.yml`)

**Triggers:**
- Pull requests to `main` or `develop`
- Manual trigger via workflow_dispatch

**Jobs:**
- **SQL Linting**:
  - SQL formatting consistency
  - Checks for trailing whitespace

- **Rust Linting** (when rust-backend/ exists):
  - Formatting with `cargo fmt`
  - Clippy warnings (all enabled)
  - Unsafe code detection
  - TODO comment tracking

- **TypeScript Linting** (when ponder-indexers/ exists):
  - Prettier formatting
  - ESLint validation
  - TypeScript type checking

- **Documentation Quality**:
  - Markdown linting
  - Verifies required documentation files
  - Checks for broken links

- **Docker Compose Validation**:
  - Syntax validation
  - Security best practices check
  - Verifies pinned versions

- **Shell Script Linting**:
  - ShellCheck validation
  - Script permission checks

## Caching Strategy

All workflows implement efficient caching to speed up builds:

- **Rust**: Caches cargo registry and compiled dependencies
- **TypeScript**: Caches pnpm store
- **Docker**: GitHub container cache for built images

## Status Badges

Add these badges to your README.md:

```markdown
![CI](https://github.com/YOUR_USERNAME/api.8004.dev/workflows/CI/badge.svg)
![Security](https://github.com/YOUR_USERNAME/api.8004.dev/workflows/Security%20Scanning/badge.svg)
![Lint](https://github.com/YOUR_USERNAME/api.8004.dev/workflows/Lint/badge.svg)
```

## GitHub Secrets Required

No secrets are required for the current workflows. When deploying to production, you may need:

- `DOCKER_USERNAME` - Docker Hub username
- `DOCKER_TOKEN` - Docker Hub access token
- `DEPLOY_KEY` - SSH key for deployment
- API keys for external services (when implemented)

## Local Testing

You can test workflows locally using [act](https://github.com/nektos/act):

```bash
# Install act
brew install act  # macOS
# or
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash  # Linux

# Run CI workflow
act push

# Run security scanning
act schedule

# Run linting
act pull_request -W .github/workflows/lint.yml
```

## Workflow Maintenance

### Adding New Tests

When adding new test suites:

1. Update the appropriate job in `ci.yml`
2. Ensure tests are added to `scripts/run-tests.sh`
3. Update this README

### Future Enhancements

Planned workflow improvements:

- [ ] Deployment workflow for production
- [ ] Performance benchmarking
- [ ] Code coverage reporting
- [ ] Automated changelog generation
- [ ] Release automation
- [ ] E2E tests with testnet contracts

## Troubleshooting

### Database Tests Failing

- Ensure migrations are in correct order
- Check TimescaleDB extension is enabled
- Verify PostgreSQL version compatibility

### Rust Tests Failing

- Check Rust toolchain version
- Clear cache if dependencies are corrupt
- Verify Cargo.lock is committed

### Security Scan Failures

- Review Trivy results in Security tab
- Update dependencies with vulnerabilities
- Check Gitleaks output for false positives

### Linting Failures

- Run formatters locally: `cargo fmt`, `pnpm format`
- Fix Clippy warnings: `cargo clippy --fix`
- Update documentation as needed

## Performance Optimization

Current workflow optimizations:

- Parallel job execution where possible
- Conditional job execution based on file changes
- Aggressive caching of dependencies
- Service containers instead of Docker Compose for speed

## Contributing

When modifying workflows:

1. Test changes locally with `act` if possible
2. Create a PR and verify all checks pass
3. Document any new required secrets or environment variables
4. Update this README if adding/removing workflows

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Docker Security Best Practices](https://docs.docker.com/develop/security-best-practices/)
- [Rust CI/CD Guide](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Ponder Documentation](https://ponder.sh/docs)
