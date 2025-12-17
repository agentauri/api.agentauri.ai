# Documentation Index

**Project**: api.agentauri.ai - AgentAuri Backend Infrastructure
**Last Updated**: December 17, 2025
**Phase**: Phase 4 Complete (REST Worker + Circuit Breaker + Agent Discovery)

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
- **[operations/TROUBLESHOOTING.md](operations/TROUBLESHOOTING.md)** - Comprehensive troubleshooting guide for common issues
- **[operations/RUNBOOK.md](operations/RUNBOOK.md)** - Operations runbook for system startup/shutdown, health checks, and incident response
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

### Phase 5 Design Documents (Not Yet Implemented)
- **[protocols/mcp-integration.md](protocols/mcp-integration.md)** - Model Context Protocol for agent communication *(Phase 5)*
- **[protocols/A2A_INTEGRATION.md](protocols/A2A_INTEGRATION.md)** - Agent-to-Agent (A2A) protocol *(Phase 5)*
- **[api/QUERY_TOOLS.md](api/QUERY_TOOLS.md)** - MCP query tools (Tier 0-3) *(Phase 5)*

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

- **[../CHANGELOG.md](../CHANGELOG.md)** - Weekly changelog with all changes
- **[deployment/PRODUCTION_DEPLOYMENT_GUIDE.md](deployment/PRODUCTION_DEPLOYMENT_GUIDE.md)** - Deployment guide and environment configuration
- **[operations/PRODUCTION_READINESS_PLAN.md](operations/PRODUCTION_READINESS_PLAN.md)** - 6-week plan to achieve production readiness

### Phase 6 Design Documents (Production Deployment)
- **[security/SECRETS_MANAGEMENT.md](security/SECRETS_MANAGEMENT.md)** - Secrets management for production *(Phase 6)*
- **[security/DATABASE_ENCRYPTION.md](security/DATABASE_ENCRYPTION.md)** - Database encryption guide *(Phase 6)*

---

## 📦 Archive

Historical reports, completed work, and superseded documents:

### December 2025
- **[archive/FRONTEND_PRD.md](archive/FRONTEND_PRD.md)** - Frontend PRD draft (Nov 2024, not in scope)
- **[archive/ROADMAP.md](archive/ROADMAP.md)** - Legacy roadmap (superseded by README.md)

### Operations Archive
- **[operations/archive/2025-12-01-ponder-event-fix.md](operations/archive/2025-12-01-ponder-event-fix.md)** - Ponder event handler fixes (Week 15)
- **[operations/archive/2025-12-01-quality-plan.md](operations/archive/2025-12-01-quality-plan.md)** - Quality & security remediation plan
- **[operations/archive/2025-12-01-fixes-completed.md](operations/archive/2025-12-01-fixes-completed.md)** - Fixes completion summary
- **[operations/archive/2025-12-01-verification.md](operations/archive/2025-12-01-verification.md)** - Final verification report

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
2. [protocols/mcp-integration.md](protocols/mcp-integration.md) - MCP protocol
3. [protocols/A2A_INTEGRATION.md](protocols/A2A_INTEGRATION.md) - A2A integration

### For Operations & SRE Teams
1. [operations/RUNBOOK.md](operations/RUNBOOK.md) - Startup/shutdown procedures, health checks, incident response
2. [operations/TROUBLESHOOTING.md](operations/TROUBLESHOOTING.md) - Common issues and solutions
3. [operations/CIRCUIT_BREAKER_GUIDE.md](operations/CIRCUIT_BREAKER_GUIDE.md) - Circuit breaker patterns
4. [security/DATABASE_ENCRYPTION.md](security/DATABASE_ENCRYPTION.md) - Database security

---

## 📊 Documentation Statistics

- **Total Documents**: 60+ markdown files (Phase 4 complete)
- **Root Level**: 7 essential files (CLAUDE.md, README, CHANGELOG, etc.)
- **Architecture Docs**: 5 files
- **Authentication Docs**: 6 files (including Social Login)
- **Security Docs**: 13 files (DATABASE_ENCRYPTION, SECURITY_HEADERS, SECRETS_MANAGER, etc.)
- **Operations Docs**: 6 files (background tasks, circuit breaker, troubleshooting, runbook, link audit, event-processor integration)
- **API Documentation**: 2 files (API_DOCUMENTATION.md, trigger-examples.md)
- **Protocol Docs**: 3 files (1 implemented, 2 Phase 5 design docs)
- **Development Guides**: 5 files (setup, testing strategy, adding chains, contributing, CI/CD)
- **Component READMEs**: 7 files
- **Archived Documents**: 6 files

**Last Comprehensive Audit**: December 17, 2025 (Phase 4 Complete)
**Recent Improvement Actions**:
- December 17, 2025:
  - Added Ponder indexer status endpoints (`/api/v1/ponder/status`, `/api/v1/ponder/events`)
  - Fixed Ponder status endpoint snake_case column mapping (v1.0.12)
  - Added monitoring token bypass for rate limiting
  - Updated API_DOCUMENTATION.md with Ponder endpoints
  - Updated CLAUDE.md with Ponder API reference
- December 4, 2025:
  - Marked Week 15 as complete in CLAUDE.md (REST Worker, Circuit Breaker, Agent Discovery)
  - Archived FRONTEND_PRD.md and ROADMAP.md (superseded documents)
  - Added Phase 5/6 status markers to design documents
  - Reorganized INDEX.md with design document sections
  - Updated production readiness assessment to 65%
- December 2, 2025:
  - Added auth/SOCIAL_LOGIN.md for Google and GitHub OAuth 2.0
  - Added operations/BACKGROUND_TASKS.md for cleanup jobs documentation
  - Updated auth/AUTHENTICATION.md with brute-force protection and social auth
  - Updated rate-limiting docs with production mode (shadow vs enforcing)
  - Updated CLAUDE.md with new features and endpoints
- December 1, 2025:
  - 4 temporary reports archived to operations/archive/
  - DEPLOYMENT.md moved to deployment/PRODUCTION_DEPLOYMENT_GUIDE.md
  - PRODUCTION_ROADMAP.md moved to operations/PRODUCTION_READINESS_PLAN.md
  - RATE_LIMITING.md renamed to RATE_LIMITS_USER_GUIDE.md for clarity
  - Cross-references added between rate-limiting docs
  - Security documentation enhanced with comprehensive links
  - Archive section added to INDEX.md

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
