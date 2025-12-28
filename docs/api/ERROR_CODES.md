# API Error Codes Reference

**Project**: api.agentauri.ai
**Last Updated**: 2025-12-28

This document provides a comprehensive reference for all API error codes, HTTP status codes, and error response formats.

---

## Error Response Format

All error responses follow a consistent JSON structure:

```json
{
  "error": "error_code",
  "message": "Human-readable description",
  "details": { "optional": "context" }
}
```

**Example:**
```json
{
  "error": "validation_error",
  "message": "Validation failed: email must be valid",
  "details": null
}
```

---

## HTTP Status Codes

### Success (2xx)

| Code | Status | Usage |
|------|--------|-------|
| 200 | OK | Successful GET, PUT, PATCH, list operations |
| 201 | Created | Successful POST (resource creation) |
| 204 | No Content | Successful DELETE operations |

### Client Errors (4xx)

| Code | Status | Common Causes |
|------|--------|---------------|
| 400 | Bad Request | Invalid JSON, validation errors, missing fields |
| 401 | Unauthorized | Missing/invalid JWT or API key |
| 403 | Forbidden | Insufficient permissions for operation |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Duplicate resource (username, email, etc.) |
| 413 | Payload Too Large | Request body exceeds 1MB |
| 429 | Too Many Requests | Rate limit exceeded |

### Server Errors (5xx)

| Code | Status | Meaning |
|------|--------|---------|
| 500 | Internal Server Error | Unexpected server error |
| 503 | Service Unavailable | Dependency unavailable (DB, Redis, Stripe) |

---

## Error Codes by Category

### Authentication Errors

| Error Code | HTTP | Description | Resolution |
|------------|------|-------------|------------|
| `unauthorized` | 401 | Missing or invalid authentication | Provide valid JWT or API key |
| `invalid_format` | 401 | API key format invalid | Use `sk_live_*` or `sk_test_*` format |
| `auth_failed` | 401 | Authentication verification failed | Check credentials |
| `key_revoked` | 401 | API key has been revoked | Generate new API key |
| `key_expired` | 401 | API key has expired | Rotate or generate new key |

### Authorization Errors

| Error Code | HTTP | Description | Resolution |
|------------|------|-------------|------------|
| `forbidden` | 403 | User lacks permission | Check role permissions |
| `insufficient_permissions` | 403 | Admin/owner role required | Contact org admin |
| `missing_organization` | 400 | X-Organization-ID header missing | Include org header |

### Validation Errors

| Error Code | HTTP | Description | Resolution |
|------------|------|-------------|------------|
| `validation_error` | 400 | Request validation failed | Check field requirements |
| `bad_request` | 400 | Invalid request data | Fix malformed JSON/params |
| `invalid_challenge` | 400 | Wallet signature invalid | Regenerate nonce, re-sign |

### Resource Errors

| Error Code | HTTP | Description | Resolution |
|------------|------|-------------|------------|
| `not_found` | 404 | Resource doesn't exist | Verify resource ID |
| `conflict` | 409 | Resource already exists | Use different identifier |
| `username_exists` | 409 | Username taken | Choose different username |
| `email_exists` | 409 | Email already registered | Use different email or login |

### Rate Limiting Errors

| Error Code | HTTP | Description | Resolution |
|------------|------|-------------|------------|
| `rate_limited` | 429 | Rate limit exceeded | Wait for `Retry-After` seconds |
| `RATE_LIMITED` | 429 | A2A protocol rate limit | Reduce request frequency |

### Server Errors

| Error Code | HTTP | Description | Resolution |
|------------|------|-------------|------------|
| `internal_error` | 500 | Unexpected server error | Retry later, contact support |
| `service_unavailable` | 503 | Service dependency down | Retry later |

---

## Rate Limiting

### Response Headers

All responses include rate limit headers:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1735344000
X-RateLimit-Window: 3600
```

### Rate Limit Tiers

| Layer | Authentication | Default Limit |
|-------|---------------|---------------|
| Layer 0 | Anonymous (IP) | 10 req/hour |
| Layer 1 | API Key | 50-2000 req/hour (plan-based) |
| Layer 2 | JWT/Wallet | Per-organization limits |

### Rate Limit Error Response

```json
{
  "error": "rate_limited",
  "message": "Rate limit exceeded. Try again in 3600 seconds.",
  "retry_after": 3600,
  "limit": 1000,
  "window": 3600
}
```

---

## A2A Protocol Errors

For A2A (Agent-to-Agent) JSON-RPC endpoints:

### JSON-RPC Error Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error description",
    "data": { "context": "optional" }
  },
  "id": "request_id"
}
```

### A2A Error Codes

| Code | Name | Meaning |
|------|------|---------|
| -32700 | Parse Error | Invalid JSON received |
| -32600 | Invalid Request | Not valid JSON-RPC format |
| -32601 | Method Not Found | Method doesn't exist |
| -32602 | Invalid Params | Invalid method parameters |
| -32603 | Internal Error | Internal server error |
| -32000 | Unauthorized | Missing/invalid authentication |
| -32001 | Insufficient Credits | Not enough credits for operation |
| -32002 | Rate Limited | Too many pending tasks |
| -32003 | Task Not Found | Requested task doesn't exist |
| -32004 | Credits Not Initialized | Credits system not setup |

---

## Endpoint-Specific Errors

### Authentication (`/api/v1/auth/*`)

| Endpoint | Error | HTTP | Cause |
|----------|-------|------|-------|
| POST /register | `username_exists` | 409 | Username already taken |
| POST /register | `email_exists` | 409 | Email already registered |
| POST /login | `unauthorized` | 401 | Invalid credentials |
| GET /google | `unauthorized` | 401 | OAuth flow failed |
| POST /verify-wallet | `invalid_challenge` | 400 | Invalid signature/nonce |

### Triggers (`/api/v1/triggers/*`)

| Endpoint | Error | HTTP | Cause |
|----------|-------|------|-------|
| POST / | `validation_error` | 400 | Invalid trigger config |
| GET /{id} | `not_found` | 404 | Trigger ID not found |
| PUT /{id} | `forbidden` | 403 | Not trigger owner |
| DELETE /{id} | `not_found` | 404 | Trigger doesn't exist |

### Organizations (`/api/v1/organizations/*`)

| Endpoint | Error | HTTP | Cause |
|----------|-------|------|-------|
| POST / | `conflict` | 409 | Slug already exists |
| GET /{id} | `forbidden` | 403 | Not org member |
| PUT /{id} | `insufficient_permissions` | 403 | Admin role required |
| DELETE /{id} | `insufficient_permissions` | 403 | Owner role required |

### Billing (`/api/v1/billing/*`)

| Endpoint | Error | HTTP | Cause |
|----------|-------|------|-------|
| GET /credits | `forbidden` | 403 | Admin role required |
| POST /credits/purchase | `service_unavailable` | 503 | Stripe not configured |
| POST /subscription | `validation_error` | 400 | Invalid plan |

### Agents (`/api/v1/agents/*`)

| Endpoint | Error | HTTP | Cause |
|----------|-------|------|-------|
| POST /link | `invalid_challenge` | 400 | Signature verification failed |
| POST /link | `conflict` | 409 | Agent already linked |
| DELETE /{id}/link | `forbidden` | 403 | Not org owner |

---

## Security Notes

### Safe Error Messages

The API never exposes sensitive information in error responses:

- Database connection details are never revealed
- Internal IP addresses are hidden
- Stack traces are not sent to clients
- Configuration values are sanitized

### Generic Error Messages

Server errors return generic messages to prevent information leakage:

```json
{
  "error": "internal_error",
  "message": "Failed to process request"
}
```

### Error Logging

All errors are logged server-side with full context for debugging while clients receive sanitized responses.

---

## Best Practices for API Clients

### Error Handling

```javascript
try {
  const response = await fetch('/api/v1/triggers', { ... });

  if (!response.ok) {
    const error = await response.json();

    switch (response.status) {
      case 400:
        // Validation error - show to user
        showError(error.message);
        break;
      case 401:
        // Auth error - redirect to login
        redirectToLogin();
        break;
      case 429:
        // Rate limited - wait and retry
        await sleep(error.retry_after * 1000);
        return retry();
      case 500:
      case 503:
        // Server error - show generic message
        showError('Service temporarily unavailable');
        break;
    }
  }
} catch (e) {
  // Network error
  showError('Network connection failed');
}
```

### Retry Strategy

| HTTP Code | Retry? | Strategy |
|-----------|--------|----------|
| 400-403 | No | Fix request |
| 404 | No | Resource doesn't exist |
| 409 | No | Change identifier |
| 429 | Yes | Wait for `Retry-After` |
| 500 | Yes | Exponential backoff |
| 503 | Yes | Exponential backoff |

### Exponential Backoff

```javascript
const delays = [1000, 2000, 4000, 8000, 16000]; // ms

for (let attempt = 0; attempt < delays.length; attempt++) {
  try {
    return await makeRequest();
  } catch (e) {
    if (e.status >= 500 && attempt < delays.length - 1) {
      await sleep(delays[attempt]);
    } else {
      throw e;
    }
  }
}
```

---

## Related Documentation

- [API Documentation](../../rust-backend/crates/api-gateway/API_DOCUMENTATION.md)
- [Authentication Guide](../auth/AUTHENTICATION.md)
- [Rate Limiting](../security/RATE_LIMITING.md)
