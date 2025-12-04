# Security Model

This document describes the security model and best practices for the api.agentauri.ai authentication system.

## Threat Model

### Assets to Protect

| Asset | Sensitivity | Impact if Compromised |
|-------|-------------|----------------------|
| User credentials | Critical | Account takeover |
| API keys | Critical | Unauthorized API access |
| Wallet signatures | High | Agent impersonation |
| Agent links | High | Billing fraud |
| Credits balance | High | Financial loss |
| Query results | Medium | Data leakage |

### Threat Actors

| Actor | Motivation | Capabilities |
|-------|------------|--------------|
| Malicious user | Free API access | Valid account, script kiddie tools |
| Competitor | Data harvesting | Moderate resources |
| Criminal | Financial gain | Advanced tools, persistence |
| Nation state | Espionage | Advanced persistent threats |

### Attack Vectors

| Vector | Threat | Mitigation |
|--------|--------|------------|
| Credential stuffing | Account takeover | Rate limiting, MFA |
| API key theft | Unauthorized access | Key rotation, monitoring |
| Replay attacks | Signature reuse | Nonce management, timestamps |
| Agent impersonation | False authentication | On-chain ownership verification |
| Rate limit bypass | Resource exhaustion | Multiple limit scopes |

## Security Controls

### Authentication Security

#### Password Security (User Auth)

- **Hashing**: Argon2id with recommended parameters
- **Minimum length**: 12 characters
- **Complexity**: Uppercase, lowercase, number, special char
- **Breach check**: HaveIBeenPwned API integration (optional)

```rust
// Argon2 configuration
let config = argon2::Config {
    variant: argon2::Variant::Argon2id,
    version: argon2::Version::Version13,
    mem_cost: 65536,    // 64 MB
    time_cost: 3,
    lanes: 4,
    secret: &[],
    ad: &[],
    hash_length: 32,
};
```

#### API Key Security (Layer 1)

- **Generation**: 32 bytes from cryptographically secure RNG
- **Storage**: Argon2 hash only; original never stored
- **Transmission**: TLS 1.3 required; HTTPS only
- **Rotation**: Grace period for seamless transition
- **Monitoring**: Last-used tracking, anomaly detection

#### Signature Security (Layer 2)

- **Standard**: EIP-191 personal sign
- **Nonce**: 16-byte random, single-use, 5-minute expiry
- **Timestamp**: 5-minute tolerance window
- **Verification**: On-chain ownership check

### Transport Security

- **TLS**: Version 1.3 minimum
- **HSTS**: Strict-Transport-Security header
- **Certificate**: EV certificate, pinning for mobile apps

```http
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

### Data Security

#### Encryption at Rest

| Data Type | Encryption | Key Management |
|-----------|------------|----------------|
| Passwords | Argon2 hash | N/A (one-way) |
| API keys | Argon2 hash | N/A (one-way) |
| Credit card | Stripe handles | Stripe PCI DSS |
| Wallet addresses | None (public) | N/A |
| Query results | AES-256-GCM | AWS KMS |

#### Data Classification

| Classification | Examples | Handling |
|----------------|----------|----------|
| Public | Agent profiles | No restrictions |
| Internal | Usage logs | Access controlled |
| Confidential | API keys, emails | Encrypted, audited |
| Restricted | Passwords | Hash only, never logged |

## Session Management

### JWT Token Security

- **Algorithm**: HS256 (symmetric) or RS256 (asymmetric)
- **Expiry**: 1 hour for access tokens
- **Refresh**: 7 days for refresh tokens
- **Storage**: HttpOnly, Secure, SameSite cookies (web)

```rust
pub struct JwtClaims {
    pub sub: String,           // User/org ID
    pub exp: i64,              // Expiration timestamp
    pub iat: i64,              // Issued at
    pub jti: String,           // Unique token ID (for revocation)
    pub scope: Vec<String>,    // Permissions
    pub org_id: Option<String>, // Organization context
}
```

### Token Revocation

- **Immediate**: Revoked tokens stored in Redis blacklist
- **TTL**: Blacklist entries expire when token would expire
- **Check**: Every request validates against blacklist

## Rate Limiting Security

### Defense in Depth

```
Layer 1: CDN/Edge (Cloudflare)
  ↓ 1000 req/sec per IP
Layer 2: Load Balancer (nginx)
  ↓ 100 req/sec per IP
Layer 3: Application (Redis)
  ↓ Per-tier limits
Layer 4: Database (connection pool)
  ↓ 100 connections max
```

### Anti-Automation

- **CAPTCHA**: After 5 failed attempts
- **Device fingerprinting**: Detect suspicious patterns
- **Behavioral analysis**: Unusual request patterns

## Audit Logging

### Events to Log

| Event | Log Level | Data Included |
|-------|-----------|---------------|
| Login success | INFO | User ID, IP, method |
| Login failure | WARN | IP, method, reason |
| API key created | INFO | Key ID, creator, permissions |
| API key revoked | INFO | Key ID, revoker, reason |
| Agent linked | INFO | Agent ID, account, wallet |
| Rate limit hit | WARN | Scope, current count |
| Permission denied | WARN | User, resource, action |

### Log Format

```json
{
  "timestamp": "2025-01-15T10:00:00Z",
  "level": "INFO",
  "event": "auth.login.success",
  "user_id": "usr_abc123",
  "ip": "192.168.1.1",
  "method": "password",
  "user_agent": "Mozilla/5.0...",
  "trace_id": "abc123xyz"
}
```

### Log Retention

| Log Type | Retention | Archival |
|----------|-----------|----------|
| Security events | 90 days | 7 years |
| Access logs | 30 days | 1 year |
| Error logs | 7 days | 90 days |

## Incident Response

### Severity Levels

| Level | Definition | Response Time |
|-------|------------|---------------|
| P1 | Active breach, data exposure | 15 minutes |
| P2 | Vulnerability being exploited | 1 hour |
| P3 | Security bug, no active exploit | 24 hours |
| P4 | Security improvement | Next sprint |

### Response Procedures

#### API Key Compromise

1. Immediately revoke compromised key
2. Audit all requests made with key
3. Notify affected organization
4. Generate new key for customer
5. Investigate source of compromise
6. Update security controls if needed

#### Account Takeover

1. Lock affected account
2. Revoke all sessions and tokens
3. Reset password via verified email
4. Audit account activity
5. Notify user of incident
6. Review and strengthen account security

## Compliance

### Standards Adherence

| Standard | Status | Notes |
|----------|--------|-------|
| OWASP Top 10 | Compliant | Annual review |
| SOC 2 Type II | In progress | 2025 Q3 target |
| GDPR | Compliant | EU data handling |
| PCI DSS | Stripe handles | Card data never touches our servers |

### Security Testing

| Type | Frequency | Provider |
|------|-----------|----------|
| Penetration testing | Annual | Third party |
| Vulnerability scanning | Weekly | Automated |
| Dependency audit | Daily | GitHub Dependabot |
| Code review | Every PR | Internal |

## Best Practices for Clients

### API Key Handling

```javascript
// DO: Use environment variables
const apiKey = process.env.API_8004_KEY;

// DON'T: Hardcode keys
const apiKey = "sk_live_abc123..."; // NEVER DO THIS!

// DO: Rotate keys regularly
// Set calendar reminder for 90-day rotation

// DO: Use minimal permissions
const key = await createApiKey({
  permissions: ["read"], // Only what's needed
});
```

### Signature Security

```javascript
// DO: Validate timestamps
if (Date.now() - timestamp > 5 * 60 * 1000) {
  throw new Error("Signature expired");
}

// DO: Use fresh nonces
const nonce = crypto.randomBytes(16).toString("hex");

// DON'T: Reuse nonces
const nonce = "static_nonce"; // NEVER DO THIS!
```

### Error Handling

```javascript
// DO: Handle auth errors gracefully
try {
  const result = await api.query(params);
} catch (error) {
  if (error.code === "INVALID_API_KEY") {
    // Key might be rotated, refresh from secrets manager
    refreshApiKey();
  } else if (error.code === "RATE_LIMITED") {
    // Implement exponential backoff
    await sleep(error.retry_after * 1000);
  }
}
```

## Related Documentation

- [AUTHENTICATION.md](./AUTHENTICATION.md) - Authentication system overview
- [API_KEYS.md](./API_KEYS.md) - API key management
- [WALLET_SIGNATURES.md](./WALLET_SIGNATURES.md) - Wallet authentication
- [RATE_LIMITING.md](./RATE_LIMITING.md) - Rate limiting implementation

---

**Last Updated**: November 24, 2025
