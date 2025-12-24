---
title: Discovery Endpoint
description: A2A Protocol Agent Card for service discovery
sidebar:
  order: 4
---

AgentAuri exposes a discovery endpoint compliant with the [A2A Protocol](https://github.com/google/a2a-protocol) (Agent-to-Agent), enabling automated service discovery by AI agents.

## Agent Card

The Agent Card provides metadata about AgentAuri's capabilities:

```bash
curl https://api.agentauri.ai/.well-known/agent.json
```

Response:
```json
{
  "name": "AgentAuri",
  "description": "Real-time blockchain monitoring infrastructure for the AI agent economy",
  "version": "1.0.0",
  "url": "https://api.agentauri.ai",
  "capabilities": {
    "triggers": {
      "description": "Event-driven triggers for blockchain monitoring",
      "actions": ["telegram", "rest", "mcp"]
    },
    "queries": {
      "description": "Query blockchain events and agent data",
      "tiers": [0, 1, 2, 3]
    },
    "authentication": {
      "methods": ["jwt", "api_key", "wallet_signature"]
    }
  },
  "endpoints": {
    "health": "/api/v1/health",
    "openapi": "/api/v1/openapi.json",
    "swagger": "/api-docs/"
  },
  "contact": {
    "email": "support@agentauri.ai",
    "documentation": "https://docs.agentauri.ai"
  },
  "supported_chains": [
    {
      "chain_id": 11155111,
      "name": "Ethereum Sepolia"
    },
    {
      "chain_id": 84532,
      "name": "Base Sepolia"
    },
    {
      "chain_id": 59141,
      "name": "Linea Sepolia"
    }
  ]
}
```

## A2A Protocol

The Agent-to-Agent (A2A) Protocol standardizes how AI agents discover and communicate with services.

### Key Concepts

| Concept | Description |
|---------|-------------|
| Agent Card | JSON metadata at `/.well-known/agent.json` |
| Capabilities | What the service can do |
| Endpoints | Available API entry points |
| Authentication | Supported auth methods |

### Discovery Flow

```
AI Agent
    │
    ├─1─► GET /.well-known/agent.json
    │     (Discover capabilities)
    │
    ├─2─► GET /api/v1/openapi.json
    │     (Get API schema)
    │
    └─3─► Make authenticated requests
          (Use the service)
```

## CORS Support

The discovery endpoint supports CORS for browser-based agents:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, OPTIONS
Access-Control-Allow-Headers: Content-Type
```

## OpenAPI Specification

AgentAuri provides a complete OpenAPI 3.0 specification:

```bash
curl https://api.agentauri.ai/api/v1/openapi.json
```

### Swagger UI

Interactive API documentation is available at:

```
https://api.agentauri.ai/api-docs/
```

## Security Endpoint

Security contact information follows the [security.txt](https://securitytxt.org/) standard:

```bash
curl https://api.agentauri.ai/.well-known/security.txt
```

Response:
```
Contact: security@agentauri.ai
Expires: 2025-12-31T23:59:59Z
Preferred-Languages: en
Canonical: https://api.agentauri.ai/.well-known/security.txt
```

## Integration Example

### Discovering AgentAuri from an AI Agent

```python
import requests

def discover_agent(base_url):
    """Discover an A2A-compatible agent"""
    agent_card = requests.get(f"{base_url}/.well-known/agent.json").json()

    print(f"Found agent: {agent_card['name']}")
    print(f"Description: {agent_card['description']}")
    print(f"Capabilities: {list(agent_card['capabilities'].keys())}")

    # Get OpenAPI spec for detailed API info
    openapi = requests.get(agent_card['endpoints']['openapi']).json()

    return {
        'agent_card': agent_card,
        'openapi': openapi
    }

# Discover AgentAuri
discovery = discover_agent("https://api.agentauri.ai")
```

### Using Discovered Capabilities

```python
def use_agent(discovery, api_key):
    """Use a discovered agent's capabilities"""
    agent_card = discovery['agent_card']

    # Check if triggers capability exists
    if 'triggers' in agent_card['capabilities']:
        # Create a trigger using the discovered API
        headers = {'X-API-Key': api_key}

        triggers = requests.get(
            f"{agent_card['url']}/api/v1/triggers",
            headers=headers
        ).json()

        print(f"Found {len(triggers['data'])} triggers")

    return triggers
```

## Best Practices

### For AI Agents

1. **Cache Agent Cards** - Don't fetch on every request
2. **Check Capabilities** - Verify features before using
3. **Use OpenAPI** - Generate client code from spec
4. **Handle Versions** - Support version negotiation

### For Integrators

1. **Discovery First** - Always start with agent card
2. **Validate Endpoints** - Test health endpoint
3. **Monitor Changes** - Watch for capability updates
4. **Secure Credentials** - Store API keys safely

## Related Standards

- [A2A Protocol](https://github.com/google/a2a-protocol) - Agent-to-Agent communication
- [OpenAPI 3.0](https://spec.openapis.org/oas/v3.0.0) - API specification
- [security.txt](https://securitytxt.org/) - Security contact standard
- [ERC-8004](https://eips.ethereum.org/EIPS/eip-8004) - Agent Economy Token Standard
