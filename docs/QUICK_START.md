# API Quick Start Guide

This guide will help you get started with the api.agentauri.ai API quickly, including authentication, rate limiting, and best practices.

## Table of Contents

1. [Authentication](#authentication)
2. [Rate Limiting](#rate-limiting)
3. [Making Your First Request](#making-your-first-request)
4. [Handling Rate Limits](#handling-rate-limits)
5. [Code Examples](#code-examples)
6. [A2A Protocol](#a2a-protocol-agent-to-agent-queries)
7. [Best Practices](#best-practices)

## Authentication

The API supports 3 authentication layers:

| Layer | Method | Rate Limit | Use Case |
|-------|--------|------------|----------|
| Layer 0 | Anonymous (no auth) | 10/hour | Testing, exploration |
| Layer 1 | API Key | 50-2000/hour | Production applications |
| Layer 2 | Wallet Signature | Inherits from org | Agent self-queries |

### Getting Started: Anonymous Access (Layer 0)

No authentication required - perfect for testing:

```bash
curl https://api.agentauri.ai/api/v1/queries/tier0/getAgentProfile?agentId=42
```

**Limitations**:
- 10 requests/hour per IP
- Tier 0-1 queries only
- x402 payment required (coming soon)

### Production Use: API Key (Layer 1)

1. **Register an account**:
```bash
curl -X POST https://api.agentauri.ai/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "your_username",
    "email": "you@example.com",
    "password": "secure_password_123"
  }'
```

2. **Create an organization** (using JWT from registration):
```bash
curl -X POST https://api.agentauri.ai/api/v1/organizations \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Company",
    "slug": "my-company"
  }'
```

3. **Create an API key**:
```bash
curl -X POST https://api.agentauri.ai/api/v1/api-keys?organization_id=YOUR_ORG_ID \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production API Key",
    "environment": "live",
    "key_type": "standard",
    "permissions": ["read", "write"]
  }'
```

**Save the API key immediately - it's only shown once!**

4. **Use your API key**:
```bash
curl https://api.agentauri.ai/api/v1/queries/tier0/getAgentProfile?agentId=42 \
  -H "Authorization: Bearer sk_live_YOUR_API_KEY"
```

## Rate Limiting

All requests are rate limited based on your authentication layer and query tier.

### Understanding Query Tiers

Different queries have different costs:

| Tier | Type | Cost | Examples |
|------|------|------|----------|
| 0 | Raw data | 1x | `getAgentProfile`, `getMyFeedbacks` |
| 1 | Aggregated | 2x | `getReputationSummary` |
| 2 | Analysis | 5x | `getClientAnalysis` |
| 3 | AI-powered | 10x | `getReputationReport` |

### Subscription Plans

| Plan | Requests/Hour | Query Tiers | Cost |
|------|---------------|-------------|------|
| Free | 50 | 0-1 | Free |
| Starter | 100 | 0-2 | $29/mo |
| Pro | 500 | 0-3 | $99/mo |
| Enterprise | 2000 | 0-3 | Custom |

**Example**: With a Starter plan (100 req/hour):
- 100 Tier 0 requests, OR
- 50 Tier 1 requests (50 × 2 = 100), OR
- 20 Tier 2 requests (20 × 5 = 100)

## Making Your First Request

### Step 1: Check API Health

```bash
curl https://api.agentauri.ai/api/v1/health
```

Response:
```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.1.0"
}
```

### Step 2: Make an Anonymous Query

```bash
curl -v https://api.agentauri.ai/api/v1/queries/tier0/getAgentProfile?agentId=42
```

### Step 3: Check Rate Limit Headers

Look for these headers in the response:

```http
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 9
X-RateLimit-Reset: 1732800600
X-RateLimit-Window: 3600
```

## Handling Rate Limits

### Reading Rate Limit Headers

Every response includes rate limit information:

```bash
curl -v https://api.agentauri.ai/api/v1/queries/tier0/getAgentProfile?agentId=42 2>&1 | grep -i ratelimit
```

Output:
```
< X-RateLimit-Limit: 10
< X-RateLimit-Remaining: 9
< X-RateLimit-Reset: 1732800600
< X-RateLimit-Window: 3600
```

### Handling 429 Errors

When rate limited, you'll receive:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 1847
X-RateLimit-Remaining: 0

{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 1847 seconds.",
    "retry_after": 1847,
    "limit": 10,
    "window": 3600
  }
}
```

## Code Examples

### Python with Retry Logic

```python
import time
import requests
from typing import Optional

class API8004Client:
    def __init__(self, api_key: Optional[str] = None):
        self.base_url = "https://api.agentauri.ai/api/v1"
        self.headers = {}
        if api_key:
            self.headers["Authorization"] = f"Bearer {api_key}"

    def make_request(self, endpoint: str, max_retries: int = 3):
        """Make API request with automatic retry on rate limit"""
        url = f"{self.base_url}/{endpoint}"

        for attempt in range(max_retries):
            response = requests.get(url, headers=self.headers)

            # Log rate limit info
            self._log_rate_limits(response)

            # Handle rate limit
            if response.status_code == 429:
                retry_after = int(response.headers.get('Retry-After', 60))
                print(f"Rate limited. Waiting {retry_after} seconds...")
                time.sleep(retry_after)
                continue

            # Success or non-retryable error
            response.raise_for_status()
            return response.json()

        raise Exception(f"Max retries ({max_retries}) exceeded")

    def _log_rate_limits(self, response):
        """Log rate limit information from response headers"""
        limit = int(response.headers.get('X-RateLimit-Limit', 0))
        remaining = int(response.headers.get('X-RateLimit-Remaining', 0))
        reset = int(response.headers.get('X-RateLimit-Reset', 0))

        if limit > 0:
            usage_pct = ((limit - remaining) / limit) * 100
            print(f"Rate Limit: {remaining}/{limit} ({usage_pct:.1f}% used)")

            if usage_pct > 80:
                print("WARNING: Approaching rate limit!")

# Usage
client = API8004Client(api_key="sk_live_YOUR_KEY")
data = client.make_request("queries/tier0/getAgentProfile?agentId=42")
print(data)
```

### JavaScript/Node.js with Exponential Backoff

```javascript
class API8004Client {
  constructor(apiKey = null) {
    this.baseUrl = 'https://api.agentauri.ai/api/v1';
    this.headers = apiKey ? { Authorization: `Bearer ${apiKey}` } : {};
  }

  async makeRequest(endpoint, maxRetries = 3) {
    const url = `${this.baseUrl}/${endpoint}`;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        const response = await fetch(url, { headers: this.headers });

        // Log rate limits
        this.logRateLimits(response);

        // Handle rate limit with exponential backoff
        if (response.status === 429) {
          const retryAfter = parseInt(response.headers.get('Retry-After') || '60');
          const backoff = Math.min(retryAfter, 2 ** attempt * 1000);

          console.log(`Rate limited. Attempt ${attempt + 1}/${maxRetries}. Waiting ${backoff}s...`);
          await new Promise(resolve => setTimeout(resolve, backoff * 1000));
          continue;
        }

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        return await response.json();
      } catch (error) {
        if (attempt === maxRetries - 1) throw error;
      }
    }

    throw new Error(`Max retries (${maxRetries}) exceeded`);
  }

  logRateLimits(response) {
    const limit = parseInt(response.headers.get('X-RateLimit-Limit') || '0');
    const remaining = parseInt(response.headers.get('X-RateLimit-Remaining') || '0');

    if (limit > 0) {
      const usagePct = ((limit - remaining) / limit) * 100;
      console.log(`Rate Limit: ${remaining}/${limit} (${usagePct.toFixed(1)}% used)`);

      if (usagePct > 80) {
        console.warn('WARNING: Approaching rate limit!');
      }
    }
  }
}

// Usage
const client = new API8004Client('sk_live_YOUR_KEY');
const data = await client.makeRequest('queries/tier0/getAgentProfile?agentId=42');
console.log(data);
```

### Bash Script with Rate Limit Monitoring

```bash
#!/bin/bash

API_KEY="sk_live_YOUR_KEY"
BASE_URL="https://api.agentauri.ai/api/v1"

make_request() {
  local endpoint="$1"
  local url="${BASE_URL}/${endpoint}"

  # Make request and capture headers
  response=$(curl -s -w "\n%{http_code}" \
    -H "Authorization: Bearer ${API_KEY}" \
    "${url}")

  # Parse response
  http_code=$(echo "$response" | tail -n1)
  body=$(echo "$response" | sed '$d')

  # Extract rate limit headers
  headers=$(curl -s -I -H "Authorization: Bearer ${API_KEY}" "${url}")
  limit=$(echo "$headers" | grep -i "x-ratelimit-limit:" | awk '{print $2}' | tr -d '\r')
  remaining=$(echo "$headers" | grep -i "x-ratelimit-remaining:" | awk '{print $2}' | tr -d '\r')
  reset=$(echo "$headers" | grep -i "x-ratelimit-reset:" | awk '{print $2}' | tr -d '\r')

  # Log rate limits
  if [ -n "$limit" ]; then
    usage=$((100 * (limit - remaining) / limit))
    echo "Rate Limit: ${remaining}/${limit} (${usage}% used)"

    if [ "$usage" -gt 80 ]; then
      echo "WARNING: Approaching rate limit!"
    fi
  fi

  # Handle rate limit
  if [ "$http_code" = "429" ]; then
    retry_after=$(echo "$headers" | grep -i "retry-after:" | awk '{print $2}' | tr -d '\r')
    echo "Rate limited. Waiting ${retry_after} seconds..."
    sleep "$retry_after"
    return 1
  fi

  echo "$body"
  return 0
}

# Usage with retry
max_retries=3
for i in $(seq 1 $max_retries); do
  if make_request "queries/tier0/getAgentProfile?agentId=42"; then
    break
  fi

  if [ "$i" = "$max_retries" ]; then
    echo "Max retries exceeded"
    exit 1
  fi
done
```

### Go with Context and Timeout

```go
package main

import (
    "context"
    "encoding/json"
    "fmt"
    "io"
    "net/http"
    "strconv"
    "time"
)

type API8004Client struct {
    BaseURL string
    APIKey  string
    Client  *http.Client
}

func NewClient(apiKey string) *API8004Client {
    return &API8004Client{
        BaseURL: "https://api.agentauri.ai/api/v1",
        APIKey:  apiKey,
        Client:  &http.Client{Timeout: 30 * time.Second},
    }
}

func (c *API8004Client) MakeRequest(ctx context.Context, endpoint string) (map[string]interface{}, error) {
    url := fmt.Sprintf("%s/%s", c.BaseURL, endpoint)

    req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
    if err != nil {
        return nil, err
    }

    if c.APIKey != "" {
        req.Header.Set("Authorization", "Bearer "+c.APIKey)
    }

    maxRetries := 3
    for attempt := 0; attempt < maxRetries; attempt++ {
        resp, err := c.Client.Do(req)
        if err != nil {
            return nil, err
        }
        defer resp.Body.Close()

        // Log rate limits
        c.logRateLimits(resp)

        // Handle rate limit
        if resp.StatusCode == http.StatusTooManyRequests {
            retryAfter := resp.Header.Get("Retry-After")
            seconds, _ := strconv.Atoi(retryAfter)
            if seconds == 0 {
                seconds = 60
            }

            fmt.Printf("Rate limited. Waiting %d seconds...\n", seconds)
            time.Sleep(time.Duration(seconds) * time.Second)
            continue
        }

        if resp.StatusCode != http.StatusOK {
            return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, resp.Status)
        }

        body, err := io.ReadAll(resp.Body)
        if err != nil {
            return nil, err
        }

        var result map[string]interface{}
        if err := json.Unmarshal(body, &result); err != nil {
            return nil, err
        }

        return result, nil
    }

    return nil, fmt.Errorf("max retries (%d) exceeded", maxRetries)
}

func (c *API8004Client) logRateLimits(resp *http.Response) {
    limit, _ := strconv.Atoi(resp.Header.Get("X-RateLimit-Limit"))
    remaining, _ := strconv.Atoi(resp.Header.Get("X-RateLimit-Remaining"))

    if limit > 0 {
        usagePct := float64(limit-remaining) / float64(limit) * 100
        fmt.Printf("Rate Limit: %d/%d (%.1f%% used)\n", remaining, limit, usagePct)

        if usagePct > 80 {
            fmt.Println("WARNING: Approaching rate limit!")
        }
    }
}

// Usage
func main() {
    client := NewClient("sk_live_YOUR_KEY")
    ctx := context.Background()

    data, err := client.MakeRequest(ctx, "queries/tier0/getAgentProfile?agentId=42")
    if err != nil {
        panic(err)
    }

    fmt.Printf("Response: %+v\n", data)
}
```

## A2A Protocol (Agent-to-Agent Queries)

The A2A Protocol enables AI agents to query reputation data asynchronously through a JSON-RPC 2.0 interface.

### Submit a Query Task

```bash
curl -X POST https://api.agentauri.ai/api/v1/a2a/rpc \
  -H "Authorization: Bearer sk_live_YOUR_API_KEY" \
  -H "X-Organization-Id: YOUR_ORG_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tasks/send",
    "params": {
      "task": {
        "tool": "getReputationSummary",
        "arguments": {"agentId": 42}
      }
    },
    "id": "1"
  }'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "submitted",
    "estimated_cost": "0.01 USDC"
  },
  "id": "1"
}
```

### Check Task Status

```bash
curl -X POST https://api.agentauri.ai/api/v1/a2a/rpc \
  -H "Authorization: Bearer sk_live_YOUR_API_KEY" \
  -H "X-Organization-Id: YOUR_ORG_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tasks/get",
    "params": {"task_id": "550e8400-e29b-41d4-a716-446655440000"},
    "id": "2"
  }'
```

### Stream Task Progress (SSE)

```bash
curl -N https://api.agentauri.ai/api/v1/a2a/tasks/550e8400-e29b-41d4-a716-446655440000/stream \
  -H "Authorization: Bearer sk_live_YOUR_API_KEY" \
  -H "Accept: text/event-stream"
```

### Available Tools

| Tool | Tier | Cost | Description |
|------|------|------|-------------|
| `getMyFeedbacks` | 0 | 0.001 USDC | Get feedback records |
| `getAgentProfile` | 0 | 0.001 USDC | Get agent profile |
| `getReputationSummary` | 1 | 0.01 USDC | Get aggregated stats |
| `getTrend` | 1 | 0.01 USDC | Get reputation trend |
| `getValidationHistory` | 1 | 0.01 USDC | Get validation history |
| `getReputationReport` | 3 | 0.20 USDC | AI-powered analysis |

For complete A2A documentation, see [A2A Protocol Integration](protocols/A2A_INTEGRATION.md).

---

## Best Practices

### 1. Always Check Rate Limit Headers

Before making expensive queries, check your remaining quota:

```python
def should_make_request(response):
    remaining = int(response.headers.get('X-RateLimit-Remaining', 0))
    limit = int(response.headers.get('X-RateLimit-Limit', 1))

    usage_pct = ((limit - remaining) / limit) * 100

    # Don't make expensive queries if <10% quota remaining
    if usage_pct > 90:
        return False

    return True
```

### 2. Implement Exponential Backoff

```python
import time
import random

def exponential_backoff(attempt, base_delay=1, max_delay=3600):
    """Calculate exponential backoff with jitter"""
    delay = min(base_delay * (2 ** attempt), max_delay)
    jitter = random.uniform(0, delay * 0.1)  # Add 10% jitter
    return delay + jitter

for attempt in range(5):
    response = make_request()
    if response.status_code != 429:
        break

    delay = exponential_backoff(attempt)
    time.sleep(delay)
```

### 3. Cache Responses Locally

```python
import time

class CachedAPI8004Client:
    def __init__(self, api_key):
        self.client = API8004Client(api_key)
        self.cache = {}

    def get_with_cache(self, endpoint, ttl=300):
        """Get data with client-side caching (TTL in seconds)"""
        now = time.time()

        # Check cache
        if endpoint in self.cache:
            cached_data, cached_time = self.cache[endpoint]
            if now - cached_time < ttl:
                print(f"Cache HIT: {endpoint}")
                return cached_data

        # Cache miss - fetch from API
        print(f"Cache MISS: {endpoint}")
        data = self.client.make_request(endpoint)

        # Update cache
        self.cache[endpoint] = (data, now)
        return data
```

### 4. Use Webhooks Instead of Polling

Instead of polling for updates:

```python
# DON'T DO THIS (wastes quota)
while True:
    data = client.make_request("queries/tier0/getMyFeedbacks?agentId=42")
    # Check for new feedbacks
    time.sleep(60)
```

Set up a trigger notification:

```python
# DO THIS (zero quota cost)
client.create_trigger({
    "name": "New Feedback Alert",
    "chain_id": 11155111,
    "registry": "reputation",
    "conditions": [{"field": "agent_id", "operator": "equals", "value": "42"}],
    "actions": [{"type": "webhook", "url": "https://your-app.com/webhook"}]
})
```

### 5. Monitor Usage Trends

```python
import time
from collections import deque

class RateLimitMonitor:
    def __init__(self, window_size=10):
        self.usage_history = deque(maxlen=window_size)

    def track_request(self, response):
        remaining = int(response.headers.get('X-RateLimit-Remaining', 0))
        limit = int(response.headers.get('X-RateLimit-Limit', 1))

        usage_pct = ((limit - remaining) / limit) * 100
        self.usage_history.append(usage_pct)

        avg_usage = sum(self.usage_history) / len(self.usage_history)

        if avg_usage > 75:
            print(f"WARNING: Average usage is {avg_usage:.1f}% - consider upgrading plan")

monitor = RateLimitMonitor()
response = client.make_request(endpoint)
monitor.track_request(response)
```

### 6. Optimize Query Tier Usage

```python
def get_reputation_data(agent_id, detail_level="basic"):
    """Get reputation data with appropriate tier based on need"""

    if detail_level == "basic":
        # Use Tier 0 (1x cost) for simple profile
        return client.make_request(f"queries/tier0/getAgentProfile?agentId={agent_id}")

    elif detail_level == "summary":
        # Use Tier 1 (2x cost) for aggregated stats
        return client.make_request(f"queries/tier1/getReputationSummary?agentId={agent_id}")

    elif detail_level == "analysis":
        # Use Tier 2 (5x cost) for detailed analysis
        return client.make_request(f"queries/tier2/getClientAnalysis?agentId={agent_id}")

    elif detail_level == "report":
        # Use Tier 3 (10x cost) for AI-powered insights
        return client.make_request(f"queries/tier3/getReputationReport?agentId={agent_id}")
```

### 7. Graceful Degradation

```python
def get_data_with_fallback(agent_id):
    """Try expensive query, fall back to cheaper tier on rate limit"""
    try:
        # Try Tier 3 first (most detailed)
        return client.make_request(f"queries/tier3/getReputationReport?agentId={agent_id}")
    except RateLimitError:
        print("Tier 3 rate limited, falling back to Tier 1")
        # Fall back to Tier 1
        return client.make_request(f"queries/tier1/getReputationSummary?agentId={agent_id}")
```

## Troubleshooting

### Issue: "Rate limit exceeded" immediately after request

**Cause**: You may have made too many expensive queries (high tier).

**Solution**: Check which tier your queries are using:
```bash
curl -v https://api.agentauri.ai/api/v1/queries/tier3/... 2>&1 | grep -i tier
```

### Issue: Can't make Tier 2/3 queries

**Cause**: Your plan doesn't support higher tiers.

**Solution**: Upgrade your subscription:
- Free → Starter for Tier 2 access
- Starter → Pro for Tier 3 access

### Issue: Rate limit headers missing

**Cause**: Request failed before middleware ran (e.g., 404, 500).

**Solution**: Check HTTP status code. Rate limit headers only appear on 200, 429 responses.

## Next Steps

1. Read the [full API documentation](/rust-backend/crates/api-gateway/API_DOCUMENTATION.md)
2. Explore [authentication layers](/docs/auth/AUTHENTICATION.md)
3. Review [rate limiting details](/docs/auth/RATE_LIMITING.md)
4. Check [API key management](/docs/auth/API_KEYS.md)

## Support

- GitHub Issues: https://github.com/erc-8004/api.agentauri.ai/issues
- Documentation: https://docs.agentauri.ai
- Email: support@agentauri.ai

---

**Last Updated**: December 21, 2024
**Version**: 1.1.0 (A2A Protocol added)
