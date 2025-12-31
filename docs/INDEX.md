# Documentation Index

**Project**: api.agentauri.ai - AgentAuri Backend Infrastructure
**Last Updated**: December 31, 2025
**Phase**: Phase 8 Complete - Performance Optimizations (Production at 90%+ readiness)

This index provides a comprehensive overview of all project documentation organized by category.

---

## 📋 Quick Start

- **[QUICK_START.md](QUICK_START.md)** - Get started with API usage in 4 languages (curl, Python, JavaScript, Rust)
- **[../CLAUDE.md](../CLAUDE.md)** - Complete project overview and development guidelines for Claude Code
- **[../README.md](../README.md)** - User-facing project introduction

---

## 🏗️ Architecture

### System Overview
- **[architecture/system-overview.md](architecture/system-overview.md)** - High-level architecture and component diagram
- **[architecture/event-store-integration.md](architecture/event-store-integration.md)** - Event Store and PostgreSQL NOTIFY/LISTEN pattern

### Rate Limiting
- **[rate-limiting/ARCHITECTURE.md](rate-limiting/ARCHITECTURE.md)** - Redis-based sliding window rate limiting design
- **[rate-limiting/QUICK_REFERENCE.md](rate-limiting/QUICK_REFERENCE.md)** - Quick reference for developers
- **[rate-limiting/IMPLEMENTATION.md](rate-limiting/IMPLEMENTATION.md)** - Week 13 implementation details

---

## 🔧 Operations & Reliability

- **[operations/BACKGROUND_TASKS.md](operations/BACKGROUND_TASKS.md)** - Background maintenance tasks (nonce cleanup, token expiry, etc.)
- **[operations/CIRCUIT_BREAKER_GUIDE.md](operations/CIRCUIT_BREAKER_GUIDE.md)** - Circuit breaker practical guide for trigger reliability
- **[operations/DISASTER_RECOVERY.md](operations/DISASTER_RECOVERY.md)** - Disaster recovery procedures for production
- **[operations/ROLLBACK_PROCEDURES.md](operations/ROLLBACK_PROCEDURES.md)** - ECS and database rollback guide
- **[operations/TROUBLESHOOTING.md](operations/TROUBLESHOOTING.md)** - Comprehensive troubleshooting guide for common issues
- **[operations/RUNBOOK.md](operations/RUNBOOK.md)** - Operations runbook for system startup/shutdown, health checks, and incident response
- **[operations/AUDIT_LOGGING.md](operations/AUDIT_LOGGING.md)** - A2A task audit logging for compliance and analytics
- **[operations/METRICS.md](operations/METRICS.md)** - Prometheus metrics and observability guide
- **[operations/LINK_AUDIT_REPORT.md](operations/LINK_AUDIT_REPORT.md)** - Documentation link audit report and fix recommendations
- **[../rust-backend/crates/event-processor/CIRCUIT_BREAKER_INTEGRATION.md](../rust-backend/crates/event-processor/CIRCUIT_BREAKER_INTEGRATION.md)** - Technical implementation details

---

## 🔐 Authentication & Security

- **[auth/AUTHENTICATION.md](auth/AUTHENTICATION.md)** - 3-layer authentication model (Anonymous, API Key, Wallet)
- **[auth/SOCIAL_LOGIN.md](auth/SOCIAL_LOGIN.md)** - Google and GitHub OAuth 2.0 integration
- **[auth/API_KEYS.md](auth/API_KEYS.md)** - API Key format, generation, and management
- **[auth/WALLET_SIGNATURES.md](auth/WALLET_SIGNATURES.md)** - EIP-191 wallet signature verification
- **[auth/RATE_LIMITS_USER_GUIDE.md](auth/RATE_LIMITS_USER_GUIDE.md)** - Comprehensive rate limit rules and tiers
- **[auth/SECURITY_MODEL.md](auth/SECURITY_MODEL.md)** - Security patterns and best practices
- **[../SECURITY.md](../SECURITY.md)** - Security policy and vulnerability reporting

---

## 💳 Payments & Billing

- **[payments/PAYMENT_SYSTEM.md](payments/PAYMENT_SYSTEM.md)** - Credits, Stripe integration, and subscription plans

---

## 🔌 Protocol Integrations

- **[protocols/erc-8004-integration.md](protocols/erc-8004-integration.md)** - ERC-8004 standard integration

### A2A Protocol (Implemented)
- **[protocols/A2A_INTEGRATION.md](protocols/A2A_INTEGRATION.md)** - Agent-to-Agent (A2A) protocol for async queries
- **[api/TOOL_REGISTRY.md](api/TOOL_REGISTRY.md)** - A2A tool catalog with tiers and pricing
- **[api/QUERY_TOOLS.md](api/QUERY_TOOLS.md)** - MCP query tools (Tier 0-3)

### Phase 5 Design Documents (Not Yet Implemented)
- **[protocols/mcp-integration.md](protocols/mcp-integration.md)** - Model Context Protocol for agent communication *(Phase 5)*

---

## 🗄️ Database

- **[database/schema.md](database/schema.md)** - Complete database schema reference
- **[../database/README.md](../database/README.md)** - Database setup and migration guide
- **[../database/tests/README.md](../database/tests/README.md)** - Database testing documentation

---

## 🔧 Development

- **[development/setup.md](development/setup.md)** - Local development environment setup
- **[development/TESTING_STRATEGY.md](development/TESTING_STRATEGY.md)** - Comprehensive testing strategy and best practices
- **[development/ADDING_NEW_CHAIN.md](development/ADDING_NEW_CHAIN.md)** - Guide for adding new blockchain network support
- **[../CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines and workflow
- **[../.github/workflows/README.md](../.github/workflows/README.md)** - CI/CD pipeline documentation

---

## 📡 API Reference

- **[../rust-backend/crates/api-gateway/API_DOCUMENTATION.md](../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)** - Complete REST API reference
- **[api/ERROR_CODES.md](api/ERROR_CODES.md)** - Comprehensive error codes and HTTP status reference
- **[examples/trigger-examples.md](examples/trigger-examples.md)** - Trigger configuration examples

### Ponder Indexer Endpoints
- `GET /api/v1/ponder/status` - Blockchain indexer sync status (per chain)
- `GET /api/v1/ponder/events` - Event statistics by chain and type

---

## 📦 Component Documentation

### Rust Backend
- **[../rust-backend/README.md](../rust-backend/README.md)** - Rust workspace overview and structure

### Ponder Indexers
- **[../ponder-indexers/README.md](../ponder-indexers/README.md)** - Blockchain indexer configuration
- **[../ponder-indexers/CONTRACTS.md](../ponder-indexers/CONTRACTS.md)** - Contract deployments and ABIs

---

## 📝 Project Tracking

- **[ROADMAP.md](ROADMAP.md)** - Development roadmap and upcoming phases (Phase 9-12)
- **[../CHANGELOG.md](../CHANGELOG.md)** - Weekly changelog with all changes
- **[deployment/PRODUCTION_DEPLOYMENT_GUIDE.md](deployment/PRODUCTION_DEPLOYMENT_GUIDE.md)** - Deployment guide and environment configuration
- **[operations/PRODUCTION_READINESS_PLAN.md](operations/PRODUCTION_READINESS_PLAN.md)** - 6-week plan to achieve production readiness

### Phase 6 Design Documents (Production Deployment)
- **[security/SECRETS_MANAGEMENT.md](security/SECRETS_MANAGEMENT.md)** - Secrets management for production *(Phase 6)*
- **[security/DATABASE_ENCRYPTION.md](security/DATABASE_ENCRYPTION.md)** - Database encryption guide *(Phase 6)*

---

## 🎯 By Use Case

### For New Developers
1. Start with [../README.md](../README.md) for project overview
2. Read [QUICK_START.md](QUICK_START.md) for API usage examples
3. Review [development/setup.md](development/setup.md) for local setup
4. Read [development/TESTING_STRATEGY.md](development/TESTING_STRATEGY.md) for testing guidelines
5. Learn [development/ADDING_NEW_CHAIN.md](development/ADDING_NEW_CHAIN.md) for multi-chain architecture
6. Check [../CONTRIBUTING.md](../CONTRIBUTING.md) for contribution workflow

### For API Users
1. [QUICK_START.md](QUICK_START.md) - Code examples in your language
2. [API_DOCUMENTATION.md](../rust-backend/crates/api-gateway/API_DOCUMENTATION.md) - Complete endpoint reference
3. [auth/AUTHENTICATION.md](auth/AUTHENTICATION.md) - Authentication methods
4. [auth/RATE_LIMITS_USER_GUIDE.md](auth/RATE_LIMITS_USER_GUIDE.md) - Rate limits and quotas

### For System Architects
1. [architecture/system-overview.md](architecture/system-overview.md) - Component architecture
2. [rate-limiting/ARCHITECTURE.md](rate-limiting/ARCHITECTURE.md) - Rate limiting design
3. [auth/SECURITY_MODEL.md](auth/SECURITY_MODEL.md) - Security patterns
4. [database/schema.md](database/schema.md) - Data model

### For Protocol Integrators
1. [protocols/erc-8004-integration.md](protocols/erc-8004-integration.md) - ERC-8004 events
2. [protocols/A2A_INTEGRATION.md](protocols/A2A_INTEGRATION.md) - A2A protocol integration
3. [api/TOOL_REGISTRY.md](api/TOOL_REGISTRY.md) - A2A tool catalog and pricing
4. [protocols/mcp-integration.md](protocols/mcp-integration.md) - MCP protocol (Phase 5)

### For Operations & SRE Teams
1. [operations/RUNBOOK.md](operations/RUNBOOK.md) - Startup/shutdown procedures, health checks, incident response
2. [operations/TROUBLESHOOTING.md](operations/TROUBLESHOOTING.md) - Common issues and solutions
3. [operations/METRICS.md](operations/METRICS.md) - Prometheus metrics and alerting
4. [operations/AUDIT_LOGGING.md](operations/AUDIT_LOGGING.md) - A2A task audit trail
5. [operations/CIRCUIT_BREAKER_GUIDE.md](operations/CIRCUIT_BREAKER_GUIDE.md) - Circuit breaker patterns
6. [security/DATABASE_ENCRYPTION.md](security/DATABASE_ENCRYPTION.md) - Database security

---

## 📊 Documentation Statistics

- **Total Documents**: 120+ markdown files
- **Root Level**: 7 essential files (CLAUDE.md, README, CHANGELOG, etc.)
- **Architecture Docs**: 3 files (system-overview, event-store, multi-region)
- **Authentication Docs**: 7 files (including Social Login, OAuth, SIWE)
- **Security Docs**: 17 files (encryption, secrets, OWASP, headers, etc.)
- **Operations Docs**: 10 files (runbook, troubleshooting, circuit breaker, metrics, etc.)
- **API Documentation**: 4 files (API_DOCUMENTATION.md, TOOL_REGISTRY.md, ERROR_CODES.md, etc.)
- **Protocol Docs**: 4 files (ERC-8004, A2A, MCP integration, MCP server)
- **Development Guides**: 5 files (setup, testing, adding chains, contributing, CI/CD)
- **Component READMEs**: 8 files

**Last Comprehensive Audit**: December 31, 2025 (Documentation Cleanup)
**Recent Improvement Actions**:
- December 31, 2025:
  - Deleted 8 obsolete archive files (~4,200 lines removed)
  - Updated CLAUDE.md with 25+ missing API endpoints
  - Updated CLAUDE.md with 13+ missing database tables
  - Added ROADMAP.md for next development phases
  - Removed archive section from INDEX.md
  - Updated documentation statistics
- December 30, 2025:
  - Added Phase 8 performance optimizations
  - Added performance indexes to database
  - Updated production readiness to 90%+
- December 29, 2025:
  - Fixed nonce endpoint
  - Added events endpoint for blockchain events
  - Added organization-scoped endpoints

---

## 🔍 Finding What You Need

**Search Tips**:
```bash
# Find all documentation files
find . -name "*.md" | grep -v node_modules | grep -v target

# Search for specific topics
grep -r "rate limiting" docs/

# Find API endpoint documentation
grep -r "POST /api/v1" docs/
```

**Quick Navigation**:
- Authentication: `docs/auth/`
- Architecture: `docs/architecture/`
- Rate Limiting: `docs/rate-limiting/`
- Operations: `docs/operations/`
- Security: `docs/security/`
- Protocols: `docs/protocols/`
- API Reference: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`

---

## 🔗 External References

Standards and protocols used in this project:

- **[ERC-8004](https://eips.ethereum.org/EIPS/eip-8004)** - Agent Economy Token Standard
- **[x402 Protocol](https://www.x402.org)** - HTTP-native cryptocurrency payments (v2 specification)
- **[MCP Specification](https://modelcontextprotocol.io/docs)** - Model Context Protocol for AI agents
- **[CAIP Standards](https://github.com/ChainAgnostic/CAIPs)** - Chain Agnostic Improvement Proposals

---

## 🆘 Need Help?

- **Issues**: Open an issue on GitHub
- **Questions**: Check existing documentation first
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Security**: See [SECURITY.md](../SECURITY.md) for vulnerability reporting
