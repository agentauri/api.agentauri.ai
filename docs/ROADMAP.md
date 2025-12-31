# Development Roadmap

**Last Updated**: December 31, 2025
**Current Phase**: Phase 8 Complete - Performance Optimizations
**Production Readiness**: 90%+

---

## Completed Phases

### Phase 1-4: Core Infrastructure ✅
- REST API with Actix-web
- PostgreSQL + TimescaleDB for event storage
- Redis for rate limiting and caching
- Ponder indexers for ERC-8004 events
- Basic trigger system

### Phase 5: Protocol Integrations ✅
- A2A Protocol (JSON-RPC 2.0)
- MCP Server for Claude Desktop
- ERC-8004 registry monitoring

### Phase 6: Security Hardening ✅
- AWS Secrets Manager integration
- Database encryption (column-level)
- OWASP Top 10 compliance
- Security headers implementation

### Phase 7: Auto-Scaling ✅
- ECS Fargate deployment
- Auto-scaling policies
- CloudWatch monitoring
- Multi-AZ database

### Phase 8: Performance Optimizations ✅
- Query performance indexes
- Connection pooling optimization
- Response caching
- Batch processing improvements

---

## Upcoming Phases

### Phase 9: Production Hardening
**Target**: Q1 2026
**Focus**: Reliability and disaster recovery

- [ ] Multi-region deployment (see `docs/architecture/MULTI_REGION_STRATEGY.md`)
- [ ] Cross-region database replication
- [ ] Enhanced monitoring and alerting (PagerDuty integration)
- [ ] Disaster recovery testing (quarterly DR drills)
- [ ] Load testing with production-like traffic
- [ ] Chaos engineering experiments

### Phase 10: Feature Expansion
**Target**: Q1-Q2 2026
**Focus**: New capabilities

- [ ] Additional blockchain networks
  - Arbitrum One
  - Optimism
  - Base
- [ ] Advanced trigger conditions
  - Complex boolean logic (AND/OR/NOT)
  - Time-based conditions
  - Cross-chain correlations
- [ ] Batch operations API
  - Bulk trigger creation
  - Batch agent following
- [ ] Event replay functionality
- [ ] Webhook retry policies (configurable)

### Phase 11: Developer Experience
**Target**: Q2 2026
**Focus**: SDK and tooling

- [ ] Official SDKs
  - Python SDK (`pip install agentauri`)
  - JavaScript/TypeScript SDK (`npm install @agentauri/sdk`)
  - Rust SDK (`agentauri-client` crate)
- [ ] Interactive API playground (Swagger UI enhancements)
- [ ] Webhook debugging tools
  - Request inspector
  - Retry simulator
  - Payload validator
- [ ] CLI tool for trigger management
- [ ] Improved documentation site (Starlight)

### Phase 12: Enterprise Features
**Target**: Q3 2026
**Focus**: Enterprise-grade capabilities

- [ ] SSO/SAML integration
- [ ] Role-based access control (RBAC) enhancements
- [ ] Audit log exports
- [ ] Custom SLAs per organization
- [ ] Dedicated infrastructure option
- [ ] SOC 2 Type II compliance

---

## Feature Requests & Ideas

### Under Consideration
- GraphQL API (alongside REST)
- gRPC support for high-performance clients
- Slack/Discord action types
- Email action type
- Custom webhook transformations (JSONata)
- Agent reputation scoring
- On-chain trigger execution (smart contract callbacks)

### Community Suggestions
Open an issue on GitHub to suggest new features.

---

## Release Schedule

| Version | Target Date | Focus |
|---------|-------------|-------|
| v1.1.0 | Jan 2026 | Phase 9 - Production Hardening |
| v1.2.0 | Mar 2026 | Phase 10 - Feature Expansion |
| v1.3.0 | Jun 2026 | Phase 11 - Developer Experience |
| v2.0.0 | Sep 2026 | Phase 12 - Enterprise Features |

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for how to contribute to the roadmap.

Priority is given to:
1. Security improvements
2. Performance optimizations
3. Developer experience
4. New integrations
