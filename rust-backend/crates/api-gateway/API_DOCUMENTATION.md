# API Gateway Documentation

## Overview

The API Gateway provides a RESTful API for managing triggers, conditions, and actions in the api.8004.dev multi-chain agent notification system.

**Base URL**: `http://localhost:8080/api/v1` (development)

## Security

### Authentication

The API uses JWT (JSON Web Tokens) for authentication. All endpoints except `/auth/register` and `/auth/login` require a valid JWT token in the Authorization header.

**Token Configuration**:
- Algorithm: HS256 (explicitly configured)
- Lifetime: 1 hour
- Format: `Authorization: Bearer <token>`

**Security Features**:
- Argon2 password hashing (memory-hard, side-channel resistant)
- JWT tokens expire after 1 hour
- Explicit algorithm validation (prevents algorithm confusion attacks)
- CORS whitelist (environment-based)
- Input validation on all endpoints
- JSON payload size limit: 1MB

**Rate Limiting** (Infrastructure Level):
- Authentication endpoints: 3 requests/minute per IP
- General API endpoints: 10 requests/second per IP
- Configured via nginx reverse proxy

### Common Security Errors

**401 Unauthorized**: Token missing, invalid, or expired
- Solution: Login again to get a new token

**413 Payload Too Large**: Request body exceeds 1MB
- Solution: Reduce payload size or split into multiple requests

**429 Too Many Requests**: Rate limit exceeded
- Solution: Wait before retrying, implement exponential backoff

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
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",  // Valid for 1 hour
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
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",  // Valid for 1 hour
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

**Error Responses**:
- `400 Bad Request`: Validation failed
- `401 Unauthorized`: Invalid credentials
- `403 Forbidden`: Account disabled

---

### Token Refresh

**Status**: Coming in Phase 3

Currently, JWT tokens expire after 1 hour. Users must login again to get a new token. A token refresh mechanism will be added in Phase 3 to allow extending sessions without re-entering credentials.

---

### Triggers

#### Create Trigger

Create a new trigger for the authenticated user.

**Endpoint**: `POST /api/v1/triggers`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "High Value Identity Mints",
  "description": "Alert when identity NFTs are minted",
  "chain_id": 1,
  "registry": "identity",
  "enabled": true,
  "is_stateful": false
}
```

**Validation**:
- `name`: 1-255 characters (required)
- `description`: Max 1000 characters (optional)
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
- `DB_NAME`: Database name (default: erc8004_backend)
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
