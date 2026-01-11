# API Gateway Documentation

## Overview

The API Gateway provides a RESTful API for managing triggers, conditions, and actions in the api.agentauri.ai multi-chain agent notification system.

**Base URL**: `http://localhost:8080/api/v1` (development)

## Security

### Authentication

The API implements a **3-layer authentication system**:

| Layer | Method | Use Case | Rate Limits |
|-------|--------|----------|-------------|
| **Layer 0** | Anonymous | Public data, [x402](https://www.x402.org) payments | 10 calls/hour per IP |
| **Layer 1** | API Key | Account-based access | Per-plan (100-2000/hr) |
| **Layer 2** | JWT + Wallet Signature | Full user/agent access | Per-account limits |

#### JWT Authentication (Layer 2)

Most endpoints require a valid JWT token in the Authorization header:

**Token Configuration**:
- Algorithm: HS256 (explicitly configured)
- Lifetime: 1 hour
- Format: `Authorization: Bearer <token>`

#### API Key Authentication (Layer 1)

For programmatic access, use API keys:
- Format: `sk_live_xxx` (production) or `sk_test_xxx` (testing)
- Header: `X-API-Key: sk_live_xxxxxxxxxxxxx`
- Created via `/api/v1/api-keys` endpoint

See [Authentication Documentation](../../../docs/auth/AUTHENTICATION.md) for complete details.

**Security Features**:
- Argon2 password hashing (memory-hard, side-channel resistant)
- JWT tokens expire after 1 hour
- Explicit algorithm validation (prevents algorithm confusion attacks)
- CORS whitelist (environment-based)
- Input validation on all endpoints
- JSON payload size limit: 1MB

**Rate Limiting**:
- Layer 0: 10 requests/hour per IP
- Layer 1: Per-plan limits (Free: 50/hr, Starter: 100/hr, Pro: 500/hr, Enterprise: 2000/hr)
- Layer 2: Per-account limits based on subscription
- Query tier cost multipliers apply (Tier 0: 1x, Tier 1: 2x, Tier 2: 5x, Tier 3: 10x)

See [Rate Limiting](#rate-limiting) section for detailed information.

### Common Security Errors

**401 Unauthorized**: Token missing, invalid, or expired
- Solution: Login again to get a new token

**413 Payload Too Large**: Request body exceeds 1MB
- Solution: Reduce payload size or split into multiple requests

**429 Too Many Requests**: Rate limit exceeded
- Solution: Wait before retrying, implement exponential backoff
- Check `X-RateLimit-Reset` header to know when limit resets
- See [Rate Limiting](#rate-limiting) for details

---

## Authentication

All endpoints except authentication endpoints require JWT authentication.

### Headers

```
Authorization: Bearer <jwt_token>
```

## Endpoints

### Authentication

#### Register User

Create a new user account.

**Endpoint**: `POST /api/v1/auth/register`

**Request Body**:
```json
{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "secure_password_123"
}
```

**Validation**:
- `username`: 3-50 characters
- `email`: Valid email format
- `password`: 8-100 characters

**Response** (201 Created):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "urt_abc123...",
  "expires_in": 3600,
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "john_doe",
    "email": "john@example.com",
    "created_at": "2024-01-15T10:30:00Z",
    "last_login_at": null,
    "is_active": true
  }
}
```

| Field | Description |
|-------|-------------|
| `token` | JWT access token (valid for 1 hour) |
| `refresh_token` | Refresh token for obtaining new access tokens (valid for 30 days) |
| `expires_in` | Access token validity in seconds |

**Error Responses**:
- `400 Bad Request`: Validation failed
- `409 Conflict`: Username or email already exists

---

#### Login

Authenticate and receive JWT token.

**Endpoint**: `POST /api/v1/auth/login`

**Request Body**:
```json
{
  "username_or_email": "john_doe",
  "password": "secure_password_123"
}
```

**Response** (200 OK):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "urt_abc123...",
  "expires_in": 3600,
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "john_doe",
    "email": "john@example.com",
    "created_at": "2024-01-15T10:30:00Z",
    "last_login_at": "2024-01-16T14:20:00Z",
    "is_active": true
  }
}
```

| Field | Description |
|-------|-------------|
| `token` | JWT access token (valid for 1 hour) |
| `refresh_token` | Refresh token for obtaining new access tokens (valid for 30 days) |
| `expires_in` | Access token validity in seconds |

**Error Responses**:
- `400 Bad Request`: Validation failed
- `401 Unauthorized`: Invalid credentials
- `403 Forbidden`: Account disabled

---

#### Refresh Token

Exchange a refresh token for a new access token. Implements token rotation: the old refresh token is invalidated and a new one is returned.

**Endpoint**: `POST /api/v1/auth/refresh`

**Request Body**:
```json
{
  "refresh_token": "urt_abc123..."
}
```

**Response** (200 OK):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "urt_xyz789...",
  "expires_in": 3600
}
```

**Security Features**:
- **Token Rotation**: Old refresh token is invalidated when a new one is issued
- **30-day Expiry**: Refresh tokens expire after 30 days
- **Device Tracking**: User agent and IP address are logged for security auditing

**Error Responses**:
- `400 Bad Request`: Validation failed
- `401 Unauthorized`: Invalid or expired refresh token

---

### Social OAuth Login

The API supports OAuth 2.0 login with Google and GitHub. These endpoints implement the authorization code flow and handle user registration, account linking, and login automatically.

#### Google OAuth - Initiate

Redirect users to Google's OAuth consent screen.

**Endpoint**: `GET /api/v1/auth/google`

**No authentication required**

**Query Parameters**:
- `redirect_after` (optional): URL path to redirect after successful login (e.g., `/dashboard`)

**Response**: `302 Found` - Redirects to Google OAuth consent screen

**Example**:
```bash
# Redirect users to this URL in your frontend
curl -I "http://localhost:8080/api/v1/auth/google?redirect_after=/dashboard"

# Response:
# HTTP/1.1 302 Found
# Location: https://accounts.google.com/o/oauth2/v2/auth?client_id=...
```

**Frontend Integration**:
```javascript
// Redirect user to initiate Google login
window.location.href = '/api/v1/auth/google?redirect_after=/dashboard';
```

---

#### Google OAuth - Callback

Handle the OAuth callback from Google. This endpoint is called by Google after user authorization.

**Endpoint**: `GET /api/v1/auth/google/callback`

**No authentication required** (callback from Google)

**Query Parameters**:
- `code`: Authorization code from Google
- `state`: CSRF protection state (generated during initiation)

**Response**: `302 Found` - Redirects to frontend with JWT token

**Success Redirect**:
```
{frontend_url}/dashboard?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Error Redirect**:
```
{frontend_url}/auth/error?code=session_expired&message=Your%20session%20has%20expired
```

**Behavior**:
1. **Existing Identity**: If the Google account is already linked → logs in the user
2. **Existing Email**: If email matches an existing user → links Google identity and logs in
3. **New User**: Creates new user account, personal organization, and links Google identity

**Error Codes**:
- `session_expired`: OAuth state expired or invalid (try again)
- `auth_failed`: Failed to authenticate with Google
- `email_required`: Google account has no verified email
- `email_not_verified`: Email not verified with Google

---

#### GitHub OAuth - Initiate

Redirect users to GitHub's OAuth consent screen.

**Endpoint**: `GET /api/v1/auth/github`

**No authentication required**

**Query Parameters**:
- `redirect_after` (optional): URL path to redirect after successful login

**Response**: `302 Found` - Redirects to GitHub OAuth consent screen

**Example**:
```bash
curl -I "http://localhost:8080/api/v1/auth/github?redirect_after=/settings"

# Response:
# HTTP/1.1 302 Found
# Location: https://github.com/login/oauth/authorize?client_id=...
```

---

#### GitHub OAuth - Callback

Handle the OAuth callback from GitHub.

**Endpoint**: `GET /api/v1/auth/github/callback`

**No authentication required** (callback from GitHub)

**Query Parameters**:
- `code`: Authorization code from GitHub
- `state`: CSRF protection state

**Response**: `302 Found` - Redirects to frontend with JWT token

**Behavior**: Same as Google callback - handles login, account linking, or new user creation.

---

### Token Refresh

**Endpoint**: `POST /api/v1/auth/refresh`

**Headers**: `Content-Type: application/json`

**Request Body**:
```json
{
  "refresh_token": "rt_abc123..."
}
```

**Response** (200 OK):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "rt_new_token...",
  "expires_in": 3600
}
```

JWT tokens expire after 1 hour. Use the refresh token to obtain a new access token without re-entering credentials. Refresh tokens are single-use and automatically rotated on each refresh.

---

### Triggers

#### Create Trigger

Create a new trigger within an organization. Triggers are organization-scoped resources.

**Endpoint**: `POST /api/v1/triggers`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "High Value Identity Mints",
  "description": "Alert when identity NFTs are minted",
  "organization_id": "550e8400-e29b-41d4-a716-446655440002",
  "chain_id": 1,
  "registry": "identity",
  "enabled": true,
  "is_stateful": false
}
```

**Validation**:
- `name`: 1-255 characters (required)
- `description`: Max 1000 characters (optional)
- `organization_id`: Valid organization UUID (required, user must be member)
- `chain_id`: Integer (required)
- `registry`: Must be one of: `identity`, `reputation`, `validation` (required)
- `enabled`: Boolean (default: true)
- `is_stateful`: Boolean (default: false)

**Response** (201 Created):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "organization_id": "550e8400-e29b-41d4-a716-446655440002",
    "name": "High Value Identity Mints",
    "description": "Alert when identity NFTs are minted",
    "chain_id": 1,
    "registry": "identity",
    "enabled": true,
    "is_stateful": false,
    "created_at": "2024-01-16T15:00:00Z",
    "updated_at": "2024-01-16T15:00:00Z"
  }
}
```

---

#### List Triggers

Get all triggers for the authenticated user with pagination.

**Endpoint**: `GET /api/v1/triggers?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `limit`: Number of results (1-100, default: 20)
- `offset`: Number of results to skip (default: 0)

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "High Value Identity Mints",
      "description": "Alert when identity NFTs are minted",
      "chain_id": 1,
      "registry": "identity",
      "enabled": true,
      "is_stateful": false,
      "created_at": "2024-01-16T15:00:00Z",
      "updated_at": "2024-01-16T15:00:00Z"
    }
  ],
  "pagination": {
    "total": 42,
    "limit": 20,
    "offset": 0,
    "has_more": true
  }
}
```

---

#### Get Trigger Details

Get a single trigger with all conditions and actions.

**Endpoint**: `GET /api/v1/triggers/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "High Value Identity Mints",
    "description": "Alert when identity NFTs are minted",
    "chain_id": 1,
    "registry": "identity",
    "enabled": true,
    "is_stateful": false,
    "created_at": "2024-01-16T15:00:00Z",
    "updated_at": "2024-01-16T15:00:00Z",
    "conditions": [
      {
        "id": 1,
        "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
        "condition_type": "event",
        "field": "event_type",
        "operator": "equals",
        "value": "AgentMinted",
        "config": null,
        "created_at": "2024-01-16T15:05:00Z"
      }
    ],
    "actions": [
      {
        "id": 1,
        "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
        "action_type": "telegram",
        "priority": 0,
        "config": {
          "chat_id": "123456789",
          "message_template": "New agent minted: {{agent_id}}"
        },
        "created_at": "2024-01-16T15:10:00Z"
      }
    ]
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger not found or doesn't belong to user

---

#### Update Trigger

Update an existing trigger. All fields are optional.

**Endpoint**: `PUT /api/v1/triggers/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "Updated Trigger Name",
  "enabled": false
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Updated Trigger Name",
    "description": "Alert when identity NFTs are minted",
    "chain_id": 1,
    "registry": "identity",
    "enabled": false,
    "is_stateful": false,
    "created_at": "2024-01-16T15:00:00Z",
    "updated_at": "2024-01-16T16:00:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger not found or doesn't belong to user

---

#### Delete Trigger

Delete a trigger and all associated conditions and actions (cascading delete).

**Endpoint**: `DELETE /api/v1/triggers/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `404 Not Found`: Trigger not found or doesn't belong to user

---

### Circuit Breaker Management

Each trigger has an associated circuit breaker that protects external services from cascading failures. The circuit breaker tracks action execution failures and can automatically disable trigger actions when failure thresholds are exceeded.

#### Get Circuit Breaker State

View the current circuit breaker state and configuration for a trigger.

**Endpoint**: `GET /api/v1/triggers/{id}/circuit-breaker`

**Headers**:
- `Authorization: Bearer <token>` or `X-API-Key: sk_live_...`
- `X-Organization-ID: <organization_id>` (required)

**Response** (200 OK):
```json
{
  "data": {
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "trigger_name": "Low Score Alert",
    "state": "Closed",
    "failure_count": 3,
    "last_failure_time": "2025-12-02T10:00:00Z",
    "opened_at": null,
    "half_open_calls": 0,
    "config": {
      "failure_threshold": 10,
      "recovery_timeout_seconds": 3600,
      "half_open_max_calls": 1
    }
  }
}
```

**Circuit States**:
| State | Description | Actions |
|-------|-------------|---------|
| `Closed` | Normal operation | All actions execute |
| `Open` | Circuit tripped due to failures | Actions blocked |
| `HalfOpen` | Testing recovery | Limited calls allowed |

**Example**:
```bash
curl -X GET "http://localhost:8080/api/v1/triggers/trigger_123/circuit-breaker" \
  -H "Authorization: Bearer eyJhbG..." \
  -H "X-Organization-ID: org_456"
```

**Error Responses**:
- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: Not a member of the organization
- `404 Not Found`: Trigger not found

---

#### Update Circuit Breaker Configuration

Update the circuit breaker configuration for a trigger. Requires write access (admin or member role).

**Endpoint**: `PATCH /api/v1/triggers/{id}/circuit-breaker`

**Headers**:
- `Authorization: Bearer <token>` or `X-API-Key: sk_live_...`
- `X-Organization-ID: <organization_id>` (required)

**Request Body** (all fields optional):
```json
{
  "failure_threshold": 20,
  "recovery_timeout_seconds": 7200,
  "half_open_max_calls": 3
}
```

**Validation**:
- `failure_threshold`: 1-1000 (number of failures before opening circuit)
- `recovery_timeout_seconds`: 60-604800 (1 minute to 7 days)
- `half_open_max_calls`: 1-10 (test calls before full recovery)

**Response** (200 OK): Returns updated circuit breaker state

**Example**:
```bash
# Increase failure threshold and recovery timeout
curl -X PATCH "http://localhost:8080/api/v1/triggers/trigger_123/circuit-breaker" \
  -H "Authorization: Bearer eyJhbG..." \
  -H "X-Organization-ID: org_456" \
  -H "Content-Type: application/json" \
  -d '{
    "failure_threshold": 20,
    "recovery_timeout_seconds": 7200
  }'
```

**Error Responses**:
- `400 Bad Request`: Invalid configuration values
- `403 Forbidden`: Insufficient permissions (viewer role)
- `404 Not Found`: Trigger not found

---

#### Reset Circuit Breaker

Manually reset a tripped circuit breaker to Closed state. This clears the failure count and allows actions to execute immediately. Requires write access.

**Endpoint**: `POST /api/v1/triggers/{id}/circuit-breaker/reset`

**Headers**:
- `Authorization: Bearer <token>` or `X-API-Key: sk_live_...`
- `X-Organization-ID: <organization_id>` (required)

**Response** (200 OK):
```json
{
  "data": {
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "trigger_name": "Low Score Alert",
    "state": "Closed",
    "failure_count": 0,
    "last_failure_time": null,
    "opened_at": null,
    "half_open_calls": 0,
    "config": {
      "failure_threshold": 10,
      "recovery_timeout_seconds": 3600,
      "half_open_max_calls": 1
    }
  }
}
```

**Example**:
```bash
# Reset a tripped circuit breaker
curl -X POST "http://localhost:8080/api/v1/triggers/trigger_123/circuit-breaker/reset" \
  -H "Authorization: Bearer eyJhbG..." \
  -H "X-Organization-ID: org_456"
```

**Use Cases**:
- After fixing the underlying issue causing failures
- To immediately resume action execution after maintenance
- To test trigger actions after configuration changes

**Error Responses**:
- `403 Forbidden`: Insufficient permissions (viewer role)
- `404 Not Found`: Trigger not found

---

### Trigger Conditions

#### Create Condition

Add a new condition to a trigger.

**Endpoint**: `POST /api/v1/triggers/{trigger_id}/conditions`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "condition_type": "event",
  "field": "event_type",
  "operator": "equals",
  "value": "AgentMinted",
  "config": {
    "case_sensitive": false
  }
}
```

**Validation**:
- `condition_type`: 1-100 characters (required)
- `field`: 1-255 characters (required)
- `operator`: 1-50 characters (required)
- `value`: 1-1000 characters (required)
- `config`: JSON object (optional)

**Response** (201 Created):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "condition_type": "event",
    "field": "event_type",
    "operator": "equals",
    "value": "AgentMinted",
    "config": {
      "case_sensitive": false
    },
    "created_at": "2024-01-16T15:05:00Z"
  }
}
```

---

#### List Conditions

Get all conditions for a trigger.

**Endpoint**: `GET /api/v1/triggers/{trigger_id}/conditions`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": 1,
      "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
      "condition_type": "event",
      "field": "event_type",
      "operator": "equals",
      "value": "AgentMinted",
      "config": null,
      "created_at": "2024-01-16T15:05:00Z"
    }
  ]
}
```

---

#### Update Condition

Update an existing condition. All fields are optional.

**Endpoint**: `PUT /api/v1/triggers/{trigger_id}/conditions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "value": "AgentBurned",
  "config": {
    "case_sensitive": true
  }
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "condition_type": "event",
    "field": "event_type",
    "operator": "equals",
    "value": "AgentBurned",
    "config": {
      "case_sensitive": true
    },
    "created_at": "2024-01-16T15:05:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger or condition not found

---

#### Delete Condition

Delete a condition from a trigger.

**Endpoint**: `DELETE /api/v1/triggers/{trigger_id}/conditions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `404 Not Found`: Trigger or condition not found

---

### Trigger Actions

#### Create Action

Add a new action to a trigger.

**Endpoint**: `POST /api/v1/triggers/{trigger_id}/actions`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "action_type": "telegram",
  "priority": 0,
  "config": {
    "chat_id": "123456789",
    "message_template": "New agent minted: {{agent_id}}"
  }
}
```

**Validation**:
- `action_type`: Must be one of: `telegram`, `rest`, `mcp` (required)
- `priority`: Integer (default: 0)
- `config`: JSON object (required)

**Response** (201 Created):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "action_type": "telegram",
    "priority": 0,
    "config": {
      "chat_id": "123456789",
      "message_template": "New agent minted: {{agent_id}}"
    },
    "created_at": "2024-01-16T15:10:00Z"
  }
}
```

---

#### List Actions

Get all actions for a trigger (ordered by priority, then ID).

**Endpoint**: `GET /api/v1/triggers/{trigger_id}/actions`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": 1,
      "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
      "action_type": "telegram",
      "priority": 0,
      "config": {
        "chat_id": "123456789",
        "message_template": "New agent minted: {{agent_id}}"
      },
      "created_at": "2024-01-16T15:10:00Z"
    }
  ]
}
```

---

#### Update Action

Update an existing action. All fields are optional.

**Endpoint**: `PUT /api/v1/triggers/{trigger_id}/actions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "priority": 1,
  "config": {
    "chat_id": "987654321",
    "message_template": "Agent minted: {{agent_id}} on chain {{chain_id}}"
  }
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "action_type": "telegram",
    "priority": 1,
    "config": {
      "chat_id": "987654321",
      "message_template": "Agent minted: {{agent_id}} on chain {{chain_id}}"
    },
    "created_at": "2024-01-16T15:10:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger or action not found

---

#### Delete Action

Delete an action from a trigger.

**Endpoint**: `DELETE /api/v1/triggers/{trigger_id}/actions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `404 Not Found`: Trigger or action not found

---

### Health Check

Check API and database health.

**Endpoint**: `GET /api/v1/health`

**No authentication required**

**Response** (200 OK):
```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.1.0"
}
```

**Response** (503 Service Unavailable):
```json
{
  "status": "unhealthy",
  "database": "disconnected",
  "version": "0.1.0"
}
```

---

### Ponder Indexer Status

Monitor the blockchain indexer's sync status across all configured chains.

#### Get Ponder Status

Get the current sync status of the blockchain indexer.

**Endpoint**: `GET /api/v1/ponder/status`

**No authentication required** (but rate limited)

**Response** (200 OK):
```json
{
  "status": "partial",
  "schema": "public",
  "namespace": "7daa",
  "chains": [
    {
      "chain": "ethereumSepolia",
      "chain_id": 11155111,
      "current_block": 9861073,
      "is_synced": true
    },
    {
      "chain": "baseSepolia",
      "chain_id": 84532,
      "current_block": 35112167,
      "is_synced": true
    },
    {
      "chain": "lineaSepolia",
      "chain_id": 59141,
      "current_block": 19590681,
      "is_synced": true
    },
    {
      "chain": "polygonAmoy",
      "chain_id": 80002,
      "current_block": 0,
      "is_synced": false
    }
  ],
  "total_events": 49535,
  "last_activity_at": "1765992622"
}
```

**Status Values**:
- `healthy`: All configured chains are synced
- `partial`: Some chains are synced, others are not
- `initializing`: No chains have been indexed yet

**Error Responses**:
- `500 Internal Server Error`: Database query failed

---

#### Get Ponder Events

Get event statistics grouped by chain and event type.

**Endpoint**: `GET /api/v1/ponder/events`

**No authentication required** (but rate limited)

**Response** (200 OK):
```json
{
  "events": [
    {
      "chain_id": 11155111,
      "event_name": "AgentRegistered",
      "count": 15234
    },
    {
      "chain_id": 11155111,
      "event_name": "ReputationUpdated",
      "count": 8721
    },
    {
      "chain_id": 84532,
      "event_name": "AgentRegistered",
      "count": 12456
    }
  ],
  "total_types": 3
}
```

**Error Responses**:
- `500 Internal Server Error`: Database query failed

---

### Discovery

System metadata endpoint for discoverability.

#### Agent Card

Get system capabilities and metadata.

**Endpoint**: `GET /.well-known/agent.json`

**No authentication required** (public endpoint)

**Caching**: Response is cached for 1 hour (Redis + HTTP Cache-Control header)

**CORS**: Supports cross-origin requests (Access-Control-Allow-Origin: *)

**Response** (200 OK):
```json
{
  "name": "AgentAuri API",
  "version": "1.0.0",
  "description": "Real-time backend infrastructure for monitoring and reacting to ERC-8004 on-chain agent economy events",
  "api_version": "v1",
  "base_url": "https://api.agentauri.ai",
  "capabilities": {
    "push_layer": {
      "enabled": true,
      "features": [
        "multi_chain_monitoring",
        "programmable_triggers",
        "telegram_notifications",
        "rest_webhooks",
        "mcp_updates"
      ],
      "supported_chains": [
        {
          "chain_id": 11155111,
          "name": "Ethereum Sepolia",
          "registries": ["identity", "reputation", "validation"]
        },
        {
          "chain_id": 84532,
          "name": "Base Sepolia",
          "registries": ["identity", "reputation", "validation"]
        },
        {
          "chain_id": 59141,
          "name": "Linea Sepolia",
          "registries": ["identity", "reputation", "validation"]
        },
        {
          "chain_id": 80002,
          "name": "Polygon Amoy",
          "registries": ["identity", "reputation", "validation"]
        }
      ]
    },
    "pull_layer": {
      "enabled": true,
      "features": [
        "a2a_protocol",
        "mcp_server",
        "async_tasks"
      ],
      "endpoints": "/api/v1/a2a/*"
    },
    "authentication": {
      "methods": ["jwt", "api_key", "wallet_signature", "oauth2"],
      "oauth2_supported": true,
      "oauth2_providers": ["google", "github"]
    },
    "rate_limiting": {
      "enabled": true,
      "tiers": [
        {
          "tier": "anonymous",
          "rate_limit": "10 calls/hour",
          "authentication": "none"
        },
        {
          "tier": "starter",
          "rate_limit": "100 calls/hour",
          "authentication": "api_key"
        },
        {
          "tier": "pro",
          "rate_limit": "500 calls/hour",
          "authentication": "api_key"
        },
        {
          "tier": "enterprise",
          "rate_limit": "2000 calls/hour",
          "authentication": "api_key"
        }
      ]
    }
  },
  "endpoints": {
    "api_documentation": "https://api.agentauri.ai/docs",
    "health_check": "https://api.agentauri.ai/api/v1/health",
    "authentication": {
      "register": "https://api.agentauri.ai/api/v1/auth/register",
      "login": "https://api.agentauri.ai/api/v1/auth/login"
    },
    "triggers": "https://api.agentauri.ai/api/v1/triggers"
  },
  "contact": {
    "email": "support@agentauri.ai",
    "github": "https://github.com/agentauri/api.agentauri.ai",
    "documentation": "https://docs.agentauri.ai"
  },
  "protocol_version": "erc-8004-v1.0",
  "generated_at": "2025-01-30T12:00:00Z"
}
```

**Environment Variables**:

The following values can be customized via environment variables:
- `BASE_URL` - Base URL for the API (default: `https://api.agentauri.ai`)
- `CONTACT_EMAIL` - Support email (default: `support@agentauri.ai`)
- `CONTACT_GITHUB` - GitHub repository (default: `https://github.com/agentauri/api.agentauri.ai`)
- `DOCUMENTATION_URL` - Documentation site (default: `https://docs.agentauri.ai`)

**Use Cases**:
- Client discovery of API capabilities
- Automated integration setup
- Version compatibility checking
- Support chain and registry information
- Rate limit tier information for client-side planning

---

### Organizations

Organizations are the multi-tenant unit for grouping resources, members, and billing.

#### Create Organization

Create a new organization. The creator becomes the owner.

**Endpoint**: `POST /api/v1/organizations`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "My Company",
  "slug": "my-company",
  "description": "Optional description"
}
```

**Validation**:
- `name`: 1-100 characters (required)
- `slug`: 1-50 characters, lowercase alphanumeric and hyphens (required, unique)
- `description`: Max 500 characters (optional)

**Response** (201 Created):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "My Company",
    "slug": "my-company",
    "description": "Optional description",
    "is_personal": false,
    "created_at": "2024-11-27T10:00:00Z",
    "updated_at": "2024-11-27T10:00:00Z"
  }
}
```

**Error Responses**:
- `400 Bad Request`: Validation failed
- `409 Conflict`: Organization slug already exists

---

#### List Organizations

Get all organizations the authenticated user is a member of.

**Endpoint**: `GET /api/v1/organizations?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `limit`: Number of results (1-100, default: 20)
- `offset`: Number of results to skip (default: 0)

**Response** (200 OK):
```json
{
  "data": [
    {
      "organization": {
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "name": "My Company",
        "slug": "my-company",
        "description": null,
        "is_personal": false,
        "created_at": "2024-11-27T10:00:00Z",
        "updated_at": "2024-11-27T10:00:00Z"
      },
      "my_role": "owner"
    }
  ],
  "pagination": {
    "total": 2,
    "limit": 20,
    "offset": 0,
    "has_more": false
  }
}
```

---

#### Get Organization

Get details for a single organization.

**Endpoint**: `GET /api/v1/organizations/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": {
    "organization": {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "name": "My Company",
      "slug": "my-company",
      "description": null,
      "is_personal": false,
      "created_at": "2024-11-27T10:00:00Z",
      "updated_at": "2024-11-27T10:00:00Z"
    },
    "my_role": "owner"
  }
}
```

**Error Responses**:
- `404 Not Found`: Organization not found or not a member

---

#### Update Organization

Update organization details. Requires admin or owner role.

**Endpoint**: `PUT /api/v1/organizations/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "Updated Company Name",
  "description": "New description"
}
```

**Response** (200 OK): Returns updated organization

**Error Responses**:
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Organization not found

---

#### Delete Organization

Delete an organization. Requires owner role. Personal organizations cannot be deleted.

**Endpoint**: `DELETE /api/v1/organizations/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `400 Bad Request`: Cannot delete personal organization
- `403 Forbidden`: Only owner can delete
- `404 Not Found`: Organization not found

---

#### Transfer Ownership

Transfer organization ownership to another member.

**Endpoint**: `POST /api/v1/organizations/{id}/transfer`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "new_owner_id": "550e8400-e29b-41d4-a716-446655440002"
}
```

**Response** (200 OK): Returns updated organization

**Error Responses**:
- `400 Bad Request`: Cannot transfer to yourself or personal org
- `403 Forbidden`: Only owner can transfer
- `404 Not Found`: Organization not found

---

### Organization Members

#### Add Member

Add a user to an organization. Requires admin or owner role.

**Endpoint**: `POST /api/v1/organizations/{id}/members`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440002",
  "role": "member"
}
```

**Validation**:
- `user_id`: Valid user UUID (required)
- `role`: One of `viewer`, `member`, `admin` (cannot add as `owner`)

**Response** (201 Created):
```json
{
  "data": {
    "id": 1,
    "user_id": "550e8400-e29b-41d4-a716-446655440002",
    "username": "jane_doe",
    "email": "jane@example.com",
    "role": "member",
    "created_at": "2024-11-27T10:00:00Z"
  }
}
```

**Error Responses**:
- `400 Bad Request`: Cannot add as owner
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: User not found
- `409 Conflict`: User is already a member

---

#### List Members

Get all members of an organization. Email addresses are masked for privacy unless you are an admin/owner or viewing your own email.

**Endpoint**: `GET /api/v1/organizations/{id}/members?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": 1,
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "john_doe",
      "email": "john@example.com",
      "role": "owner",
      "created_at": "2024-11-27T10:00:00Z"
    },
    {
      "id": 2,
      "user_id": "550e8400-e29b-41d4-a716-446655440002",
      "username": "jane_doe",
      "email": "j***@e***.com",
      "role": "member",
      "created_at": "2024-11-27T11:00:00Z"
    }
  ],
  "pagination": {
    "total": 2,
    "limit": 20,
    "offset": 0,
    "has_more": false
  }
}
```

---

#### Update Member Role

Update a member's role. Requires owner role.

**Endpoint**: `PUT /api/v1/organizations/{id}/members/{user_id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "role": "admin"
}
```

**Response** (200 OK): Returns updated member

**Error Responses**:
- `400 Bad Request`: Cannot change role to owner
- `403 Forbidden`: Only owner can update roles
- `404 Not Found`: Member not found

---

#### Remove Member

Remove a member from an organization. Requires admin or owner role.

**Endpoint**: `DELETE /api/v1/organizations/{id}/members/{user_id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `400 Bad Request`: Cannot remove the owner
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Member not found

---

### API Keys

API keys provide programmatic access to the API (Layer 1 authentication).

#### Create API Key

Create a new API key. The full key is shown **only once** at creation time.

**Endpoint**: `POST /api/v1/api-keys?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "Production API Key",
  "environment": "live",
  "key_type": "standard",
  "permissions": ["read", "write"],
  "rate_limit_override": 500,
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Validation**:
- `name`: 1-100 characters (required)
- `environment`: `live` or `test` (required)
- `key_type`: `standard`, `restricted`, or `admin` (required)
- `permissions`: Array of strings (required)
- `rate_limit_override`: Integer (optional)
- `expires_at`: ISO 8601 timestamp (optional)

**Response** (201 Created):
```json
{
  "data": {
    "id": "key_550e8400",
    "key": "sk_live_abc123xyz456789...",
    "name": "Production API Key",
    "prefix": "sk_live_abc123x",
    "environment": "live",
    "key_type": "standard",
    "permissions": ["read", "write"],
    "created_at": "2024-11-27T10:00:00Z",
    "expires_at": "2025-12-31T23:59:59Z"
  }
}
```

**Security Note**: Save the `key` value immediately - it will never be shown again.

---

#### List API Keys

List all API keys for an organization. Keys are masked (only prefix shown).

**Endpoint**: `GET /api/v1/api-keys?organization_id=xxx&limit=20&offset=0&include_revoked=false`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "items": [
    {
      "id": "key_550e8400",
      "name": "Production API Key",
      "prefix": "sk_live_abc123x",
      "environment": "live",
      "key_type": "standard",
      "permissions": ["read", "write"],
      "rate_limit_override": 500,
      "last_used_at": "2024-11-27T15:30:00Z",
      "expires_at": "2025-12-31T23:59:59Z",
      "created_at": "2024-11-27T10:00:00Z",
      "created_by": "user_123",
      "is_revoked": false,
      "revoked_at": null
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20,
  "total_pages": 1
}
```

---

#### Get API Key

Get details for a specific API key.

**Endpoint**: `GET /api/v1/api-keys/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK): Returns API key details (masked)

---

#### Revoke API Key

Revoke an API key. Revoked keys cannot be used.

**Endpoint**: `DELETE /api/v1/api-keys/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body** (optional):
```json
{
  "reason": "Compromised key"
}
```

**Response** (200 OK): Returns revoked API key

**Error Responses**:
- `400 Bad Request`: Key is already revoked
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: API key not found

---

#### Rotate API Key

Rotate an API key (revoke old, create new in one transaction).

**Endpoint**: `POST /api/v1/api-keys/{id}/rotate`

**Headers**: `Authorization: Bearer <token>`

**Request Body** (optional):
```json
{
  "name": "New Key Name",
  "expires_at": "2026-01-01T00:00:00Z"
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": "new_key_id",
    "key": "sk_live_new_secret...",
    "prefix": "sk_live_new_sec",
    "old_key_id": "key_550e8400",
    "old_key_revoked_at": "2024-11-27T12:00:00Z"
  }
}
```

**Security Note**: Save the new `key` value immediately.

---

### OAuth 2.0 Clients

OAuth 2.0 clients allow third-party applications to access the API on behalf of organizations. The API supports the client_credentials and refresh_token grant types.

#### Create OAuth Client

Register a new OAuth 2.0 client for your organization. The client secret is shown **only once** at creation time.

**Endpoint**: `POST /api/v1/oauth/clients`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "client_name": "My API Integration",
  "redirect_uris": ["https://myapp.com/callback"],
  "scopes": ["read", "write"],
  "grant_types": ["client_credentials", "refresh_token"],
  "is_trusted": false
}
```

**Validation**:
- `client_name`: 1-100 characters (required)
- `redirect_uris`: Array of valid URLs (required)
- `scopes`: Array of scope strings (required)
- `grant_types`: Array of grant types (required): `client_credentials`, `refresh_token`, `authorization_code`
- `is_trusted`: Boolean (default: false)

**Response** (201 Created):
```json
{
  "data": {
    "client_id": "oac_550e8400e29b41d4a716446655440001",
    "client_secret": "ocs_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
    "client_name": "My API Integration",
    "redirect_uris": ["https://myapp.com/callback"],
    "scopes": ["read", "write"],
    "grant_types": ["client_credentials", "refresh_token"],
    "is_trusted": false,
    "created_at": "2024-11-27T10:00:00Z"
  }
}
```

**Security Note**: Save the `client_secret` immediately - it will never be shown again.

**Example**:
```bash
curl -X POST "http://localhost:8080/api/v1/oauth/clients" \
  -H "Authorization: Bearer eyJhbG..." \
  -H "Content-Type: application/json" \
  -d '{
    "client_name": "My API Integration",
    "redirect_uris": ["https://myapp.com/callback"],
    "scopes": ["read", "write"],
    "grant_types": ["client_credentials", "refresh_token"],
    "is_trusted": false
  }'
```

**Error Responses**:
- `400 Bad Request`: Validation failed
- `403 Forbidden`: Insufficient permissions

---

#### List OAuth Clients

List all OAuth clients for the authenticated user's organization.

**Endpoint**: `GET /api/v1/oauth/clients?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `limit`: Number of results (1-100, default: 20)
- `offset`: Number of results to skip (default: 0)

**Response** (200 OK):
```json
{
  "data": {
    "clients": [
      {
        "client_id": "oac_550e8400e29b41d4a716446655440001",
        "client_name": "My API Integration",
        "redirect_uris": ["https://myapp.com/callback"],
        "scopes": ["read", "write"],
        "grant_types": ["client_credentials", "refresh_token"],
        "is_trusted": false,
        "created_at": "2024-11-27T10:00:00Z"
      }
    ],
    "total": 1
  }
}
```

**Note**: Client secrets are never returned in list responses.

---

#### Delete OAuth Client

Delete an OAuth client. This will also revoke all tokens issued to this client.

**Endpoint**: `DELETE /api/v1/oauth/clients/{client_id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Example**:
```bash
curl -X DELETE "http://localhost:8080/api/v1/oauth/clients/oac_550e8400" \
  -H "Authorization: Bearer eyJhbG..."
```

**Error Responses**:
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Client not found

---

### OAuth 2.0 Token

The token endpoint issues access tokens using the OAuth 2.0 protocol.

#### Token Endpoint

Exchange client credentials or refresh tokens for access tokens.

**Endpoint**: `POST /api/v1/oauth/token`

**No authentication header required** - client authenticates with credentials in body

**Request Body (client_credentials grant)**:
```json
{
  "grant_type": "client_credentials",
  "client_id": "oac_550e8400e29b41d4a716446655440001",
  "client_secret": "ocs_a1b2c3d4e5f6...",
  "scope": "read write"
}
```

**Request Body (refresh_token grant)**:
```json
{
  "grant_type": "refresh_token",
  "client_id": "oac_550e8400e29b41d4a716446655440001",
  "client_secret": "ocs_a1b2c3d4e5f6...",
  "refresh_token": "ort_x1y2z3..."
}
```

**Response** (200 OK):
```json
{
  "access_token": "oat_abc123def456ghi789...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "ort_xyz789abc123def456...",
  "scope": "read write"
}
```

**Token Expiration**:
- Access tokens: 1 hour (3600 seconds)
- Refresh tokens: 7 days (604800 seconds)

**Example (client_credentials)**:
```bash
curl -X POST "http://localhost:8080/api/v1/oauth/token" \
  -H "Content-Type: application/json" \
  -d '{
    "grant_type": "client_credentials",
    "client_id": "oac_550e8400",
    "client_secret": "ocs_secret...",
    "scope": "read write"
  }'
```

**Example (refresh_token)**:
```bash
curl -X POST "http://localhost:8080/api/v1/oauth/token" \
  -H "Content-Type: application/json" \
  -d '{
    "grant_type": "refresh_token",
    "client_id": "oac_550e8400",
    "client_secret": "ocs_secret...",
    "refresh_token": "ort_refresh..."
  }'
```

**Supported Grant Types**:
| Grant Type | Description | Use Case |
|------------|-------------|----------|
| `client_credentials` | Authenticate with client ID and secret | Machine-to-machine (M2M) |
| `refresh_token` | Exchange refresh token for new access token | Token renewal |
| `authorization_code` | Standard OAuth flow | Coming soon |

**Error Responses**:
- `400 Bad Request`: Invalid grant_type, missing required fields, or scope not allowed
- `401 Unauthorized`: Invalid client credentials or refresh token

---

### Agent Linking

Link on-chain agent NFTs to organizations using wallet signature verification (Layer 2 authentication).

#### Link Agent

Link an agent NFT to an organization. Requires wallet signature proving ownership.

**Endpoint**: `POST /api/v1/agents/link`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "agent_id": 42,
  "chain_id": 11155111,
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "challenge": "Sign this message to authenticate with ERC-8004 API\n\nWallet: 0x1234...\nNonce: abc123...\nExpires: 2024-11-27T12:00:00Z",
  "signature": "0x..."
}
```

**Validation**:
- `agent_id`: Integer (required)
- `chain_id`: Integer (required)
- `organization_id`: Valid org UUID (required)
- `wallet_address`: 42 characters (0x + 40 hex chars)
- `challenge`: Non-empty string with nonce and expiration
- `signature`: 130-132 characters (EIP-191 signature)

**Security Flow**:
1. Verify EIP-191 signature matches wallet address
2. Check nonce hasn't been used (replay attack prevention)
3. Verify challenge hasn't expired
4. Verify on-chain ownership via IdentityRegistry.ownerOf()
5. Create agent link

**Response** (201 Created):
```json
{
  "id": "link_550e8400",
  "agent_id": 42,
  "chain_id": 11155111,
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "status": "active",
  "created_at": "2024-11-27T10:00:00Z"
}
```

**Error Responses**:
- `400 Bad Request`: Invalid signature, nonce reused, or challenge expired
- `403 Forbidden`: Wallet does not own this agent NFT
- `409 Conflict`: Agent is already linked to an organization

---

#### List Linked Agents

List all agents linked to an organization.

**Endpoint**: `GET /api/v1/agents/linked?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
[
  {
    "id": "link_550e8400",
    "agent_id": 42,
    "chain_id": 11155111,
    "organization_id": "550e8400-e29b-41d4-a716-446655440001",
    "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
    "status": "active",
    "created_at": "2024-11-27T10:00:00Z"
  }
]
```

---

#### Unlink Agent

Remove an agent link from an organization. Requires admin or owner role.

**Endpoint**: `DELETE /api/v1/agents/{agent_id}/link?chain_id=xxx&organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "message": "Agent unlinked successfully"
}
```

**Error Responses**:
- `403 Forbidden`: Agent not linked to your organization
- `404 Not Found`: Agent link not found

---

### Billing

Credit management and Stripe integration for the payment system.

#### Get Credit Balance

Get the credit balance for an organization.

**Endpoint**: `GET /api/v1/billing/credits?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "balance": 50000000,
  "balance_formatted": "50.00 USDC"
}
```

**Note**: Balance is in micro-USDC (1 USDC = 1,000,000 micro-USDC).

---

#### List Transactions

List credit transactions for an organization.

**Endpoint**: `GET /api/v1/billing/transactions?organization_id=xxx&limit=20&offset=0&transaction_type=purchase`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `organization_id`: Required
- `limit`: 1-100 (default: 20)
- `offset`: Default: 0
- `transaction_type`: Filter by type (optional): `purchase`, `usage`, `refund`, `adjustment`

**Response** (200 OK):
```json
[
  {
    "id": "tx_550e8400",
    "organization_id": "550e8400-e29b-41d4-a716-446655440001",
    "amount": 50000000,
    "transaction_type": "purchase",
    "description": "Stripe checkout purchase",
    "reference_id": "cs_test_xxx",
    "balance_after": 50000000,
    "created_at": "2024-11-27T10:00:00Z"
  }
]
```

---

#### Purchase Credits

Create a Stripe Checkout session to purchase credits.

**Endpoint**: `POST /api/v1/billing/credits/purchase`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "amount": 50,
  "success_url": "https://example.com/success",
  "cancel_url": "https://example.com/cancel"
}
```

**Validation**:
- `amount`: 1-10000 (USD, whole units)
- `success_url`: Valid URL
- `cancel_url`: Valid URL

**Response** (200 OK):
```json
{
  "checkout_url": "https://checkout.stripe.com/...",
  "session_id": "cs_test_xxx"
}
```

**Note**: Redirect user to `checkout_url` to complete payment.

---

#### Get Subscription

Get subscription details for an organization.

**Endpoint**: `GET /api/v1/billing/subscription?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "id": "sub_550e8400",
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "stripe_subscription_id": "sub_xxx",
  "stripe_customer_id": "cus_xxx",
  "plan": "pro",
  "status": "active",
  "current_period_start": "2024-11-01T00:00:00Z",
  "current_period_end": "2024-12-01T00:00:00Z",
  "created_at": "2024-11-01T00:00:00Z"
}
```

---

#### Stripe Webhook

Handle Stripe webhook events. No authentication required (uses Stripe signature verification).

**Endpoint**: `POST /api/v1/billing/webhook`

**Headers**: `Stripe-Signature: <signature>`

**Handled Events**:
- `checkout.session.completed`: Add credits to organization
- `checkout.session.async_payment_failed`: Log failure
- `customer.subscription.created`: Create subscription record
- `customer.subscription.updated`: Update subscription status
- `customer.subscription.deleted`: Cancel subscription

**Response** (200 OK):
```json
{
  "received": true
}
```

---

### A2A Protocol

The Agent-to-Agent (A2A) Protocol enables AI agents to query on-chain reputation data through a JSON-RPC 2.0 interface. This protocol is designed for programmatic access to agent reputation, feedback, and validation data.

#### JSON-RPC Endpoint

**Endpoint**: `POST /api/v1/a2a/rpc`

**Headers**:
- `Authorization: Bearer <token>` or
- `X-API-Key: <api_key>`
- `X-Organization-Id: <org_id>` (required when using API key)
- `Content-Type: application/json`

**Request Format**:
```json
{
  "jsonrpc": "2.0",
  "method": "<method_name>",
  "params": { ... },
  "id": "<request_id>"
}
```

**Response Format (Success)**:
```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": "<request_id>"
}
```

**Response Format (Error)**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error description",
    "data": { ... }
  },
  "id": "<request_id>"
}
```

---

#### tasks/send

Submit a new query task for asynchronous execution.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tasks/send",
  "params": {
    "task": {
      "tool": "getReputationSummary",
      "arguments": {
        "agentId": 42
      }
    }
  },
  "id": "req-001"
}
```

**Response** (Success):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "submitted",
    "estimated_cost": "0.01 USDC"
  },
  "id": "req-001"
}
```

**Example (curl)**:
```bash
curl -X POST http://localhost:8080/api/v1/a2a/rpc \
  -H "Authorization: Bearer <token>" \
  -H "X-Organization-Id: <org_id>" \
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

**Validation**:
- `tool`: Must be a valid tool from the Tool Catalog
- `arguments`: Max 100KB JSON payload
- Organization must have sufficient credits
- Max 100 pending tasks per organization (rate limiting)

---

#### tasks/get

Retrieve the status and result of a previously submitted task.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tasks/get",
  "params": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": "req-002"
}
```

**Response** (Task Working):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "working",
    "progress": 0.5,
    "created_at": "2024-11-15T10:30:00Z",
    "started_at": "2024-11-15T10:30:01Z"
  },
  "id": "req-002"
}
```

**Response** (Task Completed):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "completed",
    "progress": 1.0,
    "result": {
      "agent_id": 42,
      "reputation_score": 0.85,
      "total_feedbacks": 127,
      "positive_feedbacks": 112,
      "negative_feedbacks": 15,
      "computed_at": "2024-11-15T10:30:02Z"
    },
    "cost": 0.01,
    "created_at": "2024-11-15T10:30:00Z",
    "completed_at": "2024-11-15T10:30:02Z"
  },
  "id": "req-002"
}
```

**Task Status Values**:
- `submitted`: Task queued for processing
- `working`: Task is being executed
- `completed`: Task finished successfully
- `failed`: Task execution failed
- `cancelled`: Task was cancelled by user

---

#### tasks/cancel

Cancel a pending or working task.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tasks/cancel",
  "params": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": "req-003"
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "cancelled"
  },
  "id": "req-003"
}
```

**Note**: Only tasks with status `submitted` or `working` can be cancelled.

---

#### REST Convenience Endpoints

For simpler integrations, REST endpoints are also available:

**Get Task Status**:
- **Endpoint**: `GET /api/v1/a2a/tasks/{id}`
- **Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "progress": 1.0,
  "result": { ... },
  "cost": 0.01,
  "created_at": "2024-11-15T10:30:00Z",
  "completed_at": "2024-11-15T10:30:02Z"
}
```

---

#### SSE Streaming

Stream real-time task progress updates via Server-Sent Events.

**Endpoint**: `GET /api/v1/a2a/tasks/{id}/stream`

**Headers**:
- `Authorization: Bearer <token>`
- `Accept: text/event-stream`

**Event Types**:
- `progress`: Task is still being processed
- `complete`: Task finished successfully
- `error`: Task failed or was cancelled
- `timeout`: Stream reached maximum duration (5 minutes)

**Example**:
```bash
curl -N http://localhost:8080/api/v1/a2a/tasks/<task_id>/stream \
  -H "Authorization: Bearer <token>" \
  -H "Accept: text/event-stream"
```

**Event Stream**:
```
event: progress
data: {"task_id":"xxx","status":"working","progress":0.3}

event: progress
data: {"task_id":"xxx","status":"working","progress":0.7}

event: complete
data: {"task_id":"xxx","status":"completed","progress":1.0,"result":{...}}
```

**Behavior**:
- Poll interval: 2 seconds
- Max stream duration: 5 minutes (returns `timeout` event)
- Stream closes automatically on terminal status

---

#### A2A Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32700 | Parse Error | Invalid JSON received |
| -32600 | Invalid Request | JSON is not a valid JSON-RPC request |
| -32601 | Method Not Found | Method does not exist |
| -32602 | Invalid Params | Invalid method parameters |
| -32603 | Internal Error | Internal server error |
| -32000 | Unauthorized | Missing or invalid authentication |
| -32001 | Insufficient Credits | Not enough credits to execute query |
| -32002 | Rate Limited | Too many pending tasks |
| -32003 | Task Not Found | Task does not exist or not accessible |
| -32004 | Credits Not Initialized | Organization has no credits initialized |

---

### Tool Catalog

The A2A Protocol provides tools organized by tiers based on query complexity and cost.

#### Available Tools

| Tool | Tier | Cost | Description |
|------|------|------|-------------|
| `getMyFeedbacks` | Tier 0 | 0.001 USDC | Get feedback records for an agent |
| `getAgentProfile` | Tier 0 | 0.001 USDC | Get agent profile and metadata |
| `getReputationSummary` | Tier 1 | 0.01 USDC | Get aggregated reputation statistics |
| `getTrend` | Tier 1 | 0.01 USDC | Get reputation trend over time |
| `getValidationHistory` | Tier 1 | 0.01 USDC | Get validation history for an agent |
| `getReputationReport` | Tier 3 | 0.20 USDC | AI-powered reputation analysis report |

#### Tool Tiers

| Tier | Name | Description | Cost Range |
|------|------|-------------|------------|
| **Tier 0** | Raw Data | Direct database queries, lowest latency | 0.001 USDC |
| **Tier 1** | Aggregated | Computed aggregations, moderate cost | 0.01 USDC |
| **Tier 2** | Analysis | Complex analysis (future) | TBD |
| **Tier 3** | AI-Powered | LLM-enhanced analysis, highest cost | 0.20 USDC |

#### Tool Parameters

**getMyFeedbacks / getAgentProfile / getReputationSummary / getTrend / getValidationHistory**:
```json
{
  "agentId": 42
}
```

**getReputationReport**:
```json
{
  "agentId": 42,
  "includeRecommendations": true
}
```

---

### Audit Logging

All A2A task operations are logged in the `a2a_task_audit_log` table for security, compliance, and analytics.

#### Audit Event Types

| Event | Description |
|-------|-------------|
| `created` | Task was created via tasks/send |
| `started` | Task processor began execution |
| `completed` | Task finished successfully (includes cost and duration) |
| `failed` | Task execution failed (includes error message) |
| `cancelled` | Task was cancelled by user |
| `timeout` | Task execution timed out (30 second limit) |

#### Audit Actor Types

| Actor | Description |
|-------|-------------|
| `user` | Authenticated user via JWT |
| `api_key` | Request via API key (logs key prefix) |
| `system` | Background task processor |

---

## Rate Limiting

All API requests are rate limited to ensure fair usage and system stability. The rate limiting system is tier-aware, meaning different query complexity levels consume different amounts of your quota.

### Rate Limits by Authentication Layer

The API implements a 3-layer authentication system with different rate limits per layer:

| Layer | Auth Method | Rate Limit | Query Tiers Available |
|-------|-------------|------------|----------------------|
| **Layer 0** | Anonymous (IP-based) | 10 requests/hour | Tier 0-1 only |
| **Layer 1** | API Key | 50-2000 requests/hour (plan-based) | Tier 0-3 |
| **Layer 2** | Wallet Signature | Inherits from organization | Tier 0-3 + agent operations |

### Subscription Plans (Layer 1)

API Key authentication provides different rate limits based on your organization's subscription plan:

| Plan | Requests/Hour | Query Tiers | Cost | Features |
|------|---------------|-------------|------|----------|
| **Free** | 50 | Tier 0-1 | Free | Basic queries, limited access |
| **Starter** | 100 | Tier 0-2 | $29/month | Includes analysis queries |
| **Pro** | 500 | Tier 0-3 | $99/month | Full access including AI-powered queries |
| **Enterprise** | 2000 | Tier 0-3 | Custom | Custom limits, dedicated support |

### Query Tier Cost Multipliers

Different query complexity levels consume different amounts of your rate limit quota:

| Tier | Type | Cost Multiplier | Example Queries | Use Case |
|------|------|-----------------|-----------------|----------|
| **Tier 0** | Raw Data | **1x** | `getMyFeedbacks`, `getAgentProfile`, `getValidationHistory` | Direct blockchain data access |
| **Tier 1** | Aggregated | **2x** | `getReputationSummary`, `getReputationTrend` | Pre-computed statistics |
| **Tier 2** | Analysis | **5x** | `getClientAnalysis`, `compareToBaseline` | Complex calculations |
| **Tier 3** | AI-Powered | **10x** | `getReputationReport`, `analyzeDispute`, `getRootCauseAnalysis` | LLM-based insights |

**Example Calculation**:

With a **Starter plan** (100 requests/hour), you can make:
- **100 Tier 0 queries** (100 × 1 = 100 cost units), OR
- **50 Tier 1 queries** (50 × 2 = 100 cost units), OR
- **20 Tier 2 queries** (20 × 5 = 100 cost units), OR
- **10 Tier 3 queries** (10 × 10 = 100 cost units)

You can also mix tiers. For example:
- 40 Tier 0 queries (40 × 1 = 40)
- 20 Tier 1 queries (20 × 2 = 40)
- 4 Tier 2 queries (4 × 5 = 20)
- **Total: 100 cost units** ✓

### Rate Limit Response Headers

Every API response includes rate limit information in the headers:

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 73
X-RateLimit-Reset: 1732800600
X-RateLimit-Window: 3600
Content-Type: application/json

{
  "data": { ... }
}
```

| Header | Type | Description | Example |
|--------|------|-------------|---------|
| `X-RateLimit-Limit` | Integer | Maximum cost units allowed in current window | `100` |
| `X-RateLimit-Remaining` | Integer | Remaining cost units in current window | `73` |
| `X-RateLimit-Reset` | Unix timestamp | When the rate limit window resets | `1732800600` |
| `X-RateLimit-Window` | Integer | Window duration in seconds (always 3600 = 1 hour) | `3600` |

**Important**: The `Remaining` value accounts for query tier costs. A Tier 2 query (5x cost) will decrease `Remaining` by 5 units.

### Rate Limit Exceeded (429 Error)

When you exceed your rate limit, the API returns a 429 status code:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 1847
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1732802447
Content-Type: application/json

{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 1847 seconds. (Limit: 100, Window: 3600s)",
    "retry_after": 1847,
    "limit": 100,
    "window": 3600
  }
}
```

**Response Headers**:
- `Retry-After`: Seconds to wait before retrying (same as `X-RateLimit-Reset` - current time)
- `X-RateLimit-Limit`: Your rate limit ceiling
- `X-RateLimit-Remaining`: Always 0 when rate limited
- `X-RateLimit-Reset`: Unix timestamp when quota refills

### Best Practices for Rate Limiting

1. **Monitor Your Usage**
   ```bash
   # Check remaining quota before making expensive queries
   curl -H "Authorization: Bearer sk_live_..." \
     https://api.agentauri.ai/api/v1/billing/credits?organization_id=org_123
   ```

2. **Check Headers Before Retrying**
   ```python
   import time
   import requests

   response = requests.get(url, headers=headers)

   if response.status_code == 429:
       retry_after = int(response.headers['Retry-After'])
       print(f"Rate limited. Waiting {retry_after} seconds...")
       time.sleep(retry_after)
   ```

3. **Implement Exponential Backoff**
   ```javascript
   async function makeRequestWithBackoff(url, options, maxRetries = 3) {
     for (let i = 0; i < maxRetries; i++) {
       const response = await fetch(url, options);

       if (response.status !== 429) {
         return response;
       }

       const retryAfter = parseInt(response.headers.get('Retry-After'));
       const backoff = Math.min(retryAfter || (2 ** i), 3600); // Cap at 1 hour

       console.log(`Attempt ${i + 1} rate limited, waiting ${backoff}s`);
       await new Promise(resolve => setTimeout(resolve, backoff * 1000));
     }

     throw new Error('Max retries exceeded');
   }
   ```

4. **Cache Responses When Possible**
   - Store Tier 1-3 query results locally
   - Use ETags if available
   - Implement client-side caching with TTL

5. **Use Webhooks Instead of Polling**
   - Subscribe to event notifications via triggers
   - Avoid repeated queries for the same data
   - Significantly reduces API usage

6. **Optimize Query Tier Usage**
   - Use Tier 0 queries when raw data is sufficient
   - Batch Tier 3 queries (AI-powered) for periodic reports
   - Cache expensive query results

7. **Upgrade When Needed**
   - Monitor `X-RateLimit-Remaining` trends
   - Upgrade plan before consistently hitting limits
   - Consider Enterprise plan for high-volume applications

### Monitoring Rate Limit Usage

#### Check Current Usage (API)

```bash
GET /api/v1/billing/credits?organization_id=org_abc123
Authorization: Bearer sk_live_...

# Response
{
  "data": {
    "balance": 1000,
    "usage_this_hour": 45,
    "limit": 100,
    "reset_at": "2024-11-28T15:00:00Z",
    "tier_breakdown": {
      "tier0_calls": 20,
      "tier1_calls": 10,
      "tier2_calls": 3,
      "tier3_calls": 0,
      "total_cost": 45
    }
  }
}
```

#### Track Headers in Every Response

```python
import requests

def track_rate_limits(response):
    """Extract and log rate limit info from response headers"""
    limit = int(response.headers.get('X-RateLimit-Limit', 0))
    remaining = int(response.headers.get('X-RateLimit-Remaining', 0))
    reset = int(response.headers.get('X-RateLimit-Reset', 0))

    usage_percent = ((limit - remaining) / limit) * 100 if limit > 0 else 0

    print(f"Rate Limit: {remaining}/{limit} ({usage_percent:.1f}% used)")

    if usage_percent > 80:
        print("WARNING: Approaching rate limit!")

    return {
        'limit': limit,
        'remaining': remaining,
        'reset': reset,
        'usage_percent': usage_percent
    }

response = requests.get(url, headers=headers)
rate_info = track_rate_limits(response)
```

### Rate Limit FAQs

**Q: What happens if my rate limit resets while I'm being rate limited?**
A: The limit automatically resets at the Unix timestamp specified in `X-RateLimit-Reset`. You can make requests immediately after that time.

**Q: Do failed requests count toward my rate limit?**
A: Yes, all requests (successful or failed) count toward your rate limit, except for authentication failures (401) and server errors (5xx).

**Q: Can I increase my rate limit temporarily?**
A: Enterprise plans can request temporary limit increases. Contact support with your use case.

**Q: How are rate limits calculated for Layer 2 (Wallet Signature)?**
A: Agents inherit the rate limit from their linked organization. Multiple agents under one organization share the same quota.

**Q: What if Redis is unavailable?**
A: The system fails open (allows requests) with degraded mode. You'll see `X-RateLimit-Status: degraded` header. Normal limits resume when Redis recovers.

**Q: Can I monitor rate limit usage across all my API keys?**
A: Yes, all API keys within an organization share the same quota. Check `/api/v1/billing/credits` for organization-wide usage.

**Q: How do I know which tier my query uses?**
A: Check the endpoint path (`/api/v1/queries/tier0/...`) or query parameter (`?tier=2`). If not specified, defaults to Tier 0.

---

## Error Responses

All error responses follow this format:

```json
{
  "error": "error_code",
  "message": "Human-readable error message"
}
```

### Common HTTP Status Codes

- `200 OK`: Request succeeded
- `201 Created`: Resource created successfully
- `204 No Content`: Request succeeded with no response body
- `400 Bad Request`: Invalid request or validation error
- `401 Unauthorized`: Authentication required or invalid token
- `403 Forbidden`: Authenticated but not authorized
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (e.g., duplicate username)
- `500 Internal Server Error`: Server error

---

## Additional Security Details

### JWT Tokens

- JWT tokens expire after 1 hour (production hardened)
- Tokens are signed using HS256 algorithm (explicitly configured)
- Include user ID and username in claims
- Store JWT secret in `JWT_SECRET` environment variable
- Expiration validation enabled with 60-second clock skew tolerance

### Password Requirements

- Minimum 8 characters
- Maximum 100 characters
- Hashed using Argon2 (memory-hard, side-channel resistant)

### Authorization

- Users can only access/modify their own triggers
- All trigger-related operations verify ownership
- Conditions and actions inherit trigger ownership

### Production Hardening (Week 7)

The following security enhancements were implemented:

1. **JWT Algorithm Explicitly Configured**: Prevents algorithm confusion attacks
2. **Token Lifetime Reduced**: 7 days → 1 hour (168x reduction in compromise window)
3. **JSON Payload Size Limits**: 1MB maximum (prevents DoS via memory exhaustion)
4. **Infrastructure Rate Limiting**: nginx-based rate limiting on all endpoints
5. **CORS Whitelist**: Environment-based origin restrictions

---

## Configuration

### Environment Variables

Required:
- `DB_PASSWORD`: PostgreSQL password
- `JWT_SECRET`: Secret for signing JWT tokens (production)

Optional:
- `DB_HOST`: Database host (default: localhost)
- `DB_PORT`: Database port (default: 5432)
- `DB_NAME`: Database name (default: agentauri_backend)
- `DB_USER`: Database user (default: postgres)
- `DB_MAX_CONNECTIONS`: Max DB connections (default: 10)
- `SERVER_HOST`: Server host (default: 0.0.0.0)
- `SERVER_PORT`: Server port (default: 8080)
- `ALLOWED_ORIGINS`: Comma-separated CORS origins

### Running the Server

```bash
# Set environment variables
export DB_PASSWORD=your_password
export JWT_SECRET=your_secret_key

# Run the server
cd rust-backend
cargo run --bin api-gateway
```

Server will start on `http://0.0.0.0:8080` by default.

---

## Example Usage

### Complete Workflow

1. **Register a user**:
```bash
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "secure_pass_123"
  }'
```

2. **Login to get token**:
```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username_or_email": "alice",
    "password": "secure_pass_123"
  }'
```

3. **Create a trigger**:
```bash
curl -X POST http://localhost:8080/api/v1/triggers \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Identity Mints Monitor",
    "description": "Track identity NFT mints",
    "chain_id": 1,
    "registry": "identity",
    "enabled": true
  }'
```

4. **Add a condition**:
```bash
curl -X POST http://localhost:8080/api/v1/triggers/TRIGGER_ID/conditions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "condition_type": "event",
    "field": "event_type",
    "operator": "equals",
    "value": "AgentMinted"
  }'
```

5. **Add an action**:
```bash
curl -X POST http://localhost:8080/api/v1/triggers/TRIGGER_ID/actions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "priority": 0,
    "config": {
      "chat_id": "123456789",
      "message_template": "Agent {{agent_id}} minted!"
    }
  }'
```

6. **List all triggers**:
```bash
curl http://localhost:8080/api/v1/triggers?limit=10&offset=0 \
  -H "Authorization: Bearer YOUR_TOKEN"
```

7. **Get trigger details**:
```bash
curl http://localhost:8080/api/v1/triggers/TRIGGER_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```

---

## Architecture

### File Structure

```
api-gateway/src/
├── main.rs                 # Entry point and server setup
├── routes.rs               # Route configuration
├── middleware.rs           # CORS and JWT authentication
├── models/                 # Request/Response DTOs
│   ├── mod.rs
│   ├── auth.rs            # Auth DTOs
│   ├── triggers.rs        # Trigger DTOs
│   ├── conditions.rs      # Condition DTOs
│   ├── actions.rs         # Action DTOs
│   └── common.rs          # Common DTOs (errors, pagination)
├── repositories/          # Database access layer
│   ├── mod.rs
│   ├── users.rs          # User repository
│   ├── triggers.rs       # Trigger repository
│   ├── conditions.rs     # Condition repository
│   └── actions.rs        # Action repository
└── handlers/             # Request handlers
    ├── mod.rs
    ├── auth.rs          # Auth handlers
    ├── triggers.rs      # Trigger handlers
    ├── conditions.rs    # Condition handlers
    ├── actions.rs       # Action handlers
    └── health.rs        # Health check handler
```

### Design Patterns

- **Repository Pattern**: Database access abstracted into repositories
- **DTO Pattern**: Separate request/response models from database models
- **Middleware Pattern**: JWT authentication applied declaratively
- **Layered Architecture**: Clear separation between handlers, repositories, and models

---

## Testing

Health check endpoint can be used for testing:

```bash
curl http://localhost:8080/api/v1/health
```

Expected response:
```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.1.0"
}
```
