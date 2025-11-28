# Documentation Index

**Project**: api.8004.dev - ERC-8004 Backend Infrastructure
**Last Updated**: November 28, 2024

This index provides a comprehensive overview of all project documentation organized by category.

---

## 📋 Quick Start

- **[QUICK_START.md](QUICK_START.md)** - Get started with API usage in 4 languages (curl, Python, JavaScript, Rust)
- **[../CLAUDE.md](../CLAUDE.md)** - Complete project overview and development guidelines for Claude Code
- **[../README.md](../README.md)** - User-facing project introduction
- **[ROADMAP.md](ROADMAP.md)** - Development phases and weekly progress tracking

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

## 🔐 Authentication & Security

- **[auth/AUTHENTICATION.md](auth/AUTHENTICATION.md)** - 3-layer authentication model (Anonymous, API Key, Wallet)
- **[auth/API_KEYS.md](auth/API_KEYS.md)** - API Key format, generation, and management
- **[auth/WALLET_SIGNATURES.md](auth/WALLET_SIGNATURES.md)** - EIP-191 wallet signature verification
- **[auth/RATE_LIMITING.md](auth/RATE_LIMITING.md)** - Comprehensive rate limit rules and tiers
- **[auth/SECURITY_MODEL.md](auth/SECURITY_MODEL.md)** - Security patterns and best practices
- **[../SECURITY.md](../SECURITY.md)** - Security policy and vulnerability reporting

---

## 💳 Payments & Billing

- **[payments/PAYMENT_SYSTEM.md](payments/PAYMENT_SYSTEM.md)** - Credits, Stripe integration, and subscription plans

---

## 🔌 Protocol Integrations

- **[protocols/erc-8004-integration.md](protocols/erc-8004-integration.md)** - ERC-8004 standard integration
- **[protocols/mcp-integration.md](protocols/mcp-integration.md)** - Model Context Protocol for agent communication
- **[protocols/A2A_INTEGRATION.md](protocols/A2A_INTEGRATION.md)** - Agent-to-Agent (A2A) protocol

---

## 🗄️ Database

- **[database/schema.md](database/schema.md)** - Complete database schema reference
- **[../database/README.md](../database/README.md)** - Database setup and migration guide
- **[../database/tests/README.md](../database/tests/README.md)** - Database testing documentation

---

## 🔧 Development

- **[development/setup.md](development/setup.md)** - Local development environment setup
- **[../CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines and workflow
- **[../.github/workflows/README.md](../.github/workflows/README.md)** - CI/CD pipeline documentation

---

## 📡 API Reference

- **[../rust-backend/crates/api-gateway/API_DOCUMENTATION.md](../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)** - Complete REST API reference
- **[api/QUERY_TOOLS.md](api/QUERY_TOOLS.md)** - MCP query tools (Tier 0-3)
- **[examples/trigger-examples.md](examples/trigger-examples.md)** - Trigger configuration examples

---

## 📦 Component Documentation

### Rust Backend
- **[../rust-backend/README.md](../rust-backend/README.md)** - Rust workspace overview and structure

### Ponder Indexers
- **[../ponder-indexers/README.md](../ponder-indexers/README.md)** - Blockchain indexer configuration
- **[../ponder-indexers/CONTRACTS.md](../ponder-indexers/CONTRACTS.md)** - Contract deployments and ABIs

---

## 📝 Project Tracking

- **[ROADMAP.md](ROADMAP.md)** - Phase-by-phase development roadmap
- **[../CHANGELOG.md](../CHANGELOG.md)** - Weekly changelog with all changes
- **[../DEPLOYMENT.md](../DEPLOYMENT.md)** - Deployment guide and environment configuration

---

## 🎯 By Use Case

### For New Developers
1. Start with [../README.md](../README.md) for project overview
2. Read [QUICK_START.md](QUICK_START.md) for API usage examples
3. Review [development/setup.md](development/setup.md) for local setup
4. Check [../CONTRIBUTING.md](../CONTRIBUTING.md) for contribution workflow

### For API Users
1. [QUICK_START.md](QUICK_START.md) - Code examples in your language
2. [API_DOCUMENTATION.md](../rust-backend/crates/api-gateway/API_DOCUMENTATION.md) - Complete endpoint reference
3. [auth/AUTHENTICATION.md](auth/AUTHENTICATION.md) - Authentication methods
4. [auth/RATE_LIMITING.md](auth/RATE_LIMITING.md) - Rate limits and quotas

### For System Architects
1. [architecture/system-overview.md](architecture/system-overview.md) - Component architecture
2. [rate-limiting/ARCHITECTURE.md](rate-limiting/ARCHITECTURE.md) - Rate limiting design
3. [auth/SECURITY_MODEL.md](auth/SECURITY_MODEL.md) - Security patterns
4. [database/schema.md](database/schema.md) - Data model

### For Protocol Integrators
1. [protocols/erc-8004-integration.md](protocols/erc-8004-integration.md) - ERC-8004 events
2. [protocols/mcp-integration.md](protocols/mcp-integration.md) - MCP protocol
3. [protocols/A2A_INTEGRATION.md](protocols/A2A_INTEGRATION.md) - A2A integration

---

## 📊 Documentation Statistics

- **Total Documents**: 37 markdown files
- **Primary Documentation**: 11 files (this index, CLAUDE.md, README, etc.)
- **Architecture Docs**: 5 files
- **Authentication Docs**: 5 files
- **API Documentation**: 3 files
- **Protocol Docs**: 3 files
- **Development Guides**: 3 files
- **Component READMEs**: 7 files

**Last Comprehensive Audit**: November 28, 2024

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
- Protocols: `docs/protocols/`
- API Reference: `rust-backend/crates/api-gateway/API_DOCUMENTATION.md`

---

## 🆘 Need Help?

- **Issues**: Open an issue on GitHub
- **Questions**: Check existing documentation first
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Security**: See [SECURITY.md](../SECURITY.md) for vulnerability reporting
