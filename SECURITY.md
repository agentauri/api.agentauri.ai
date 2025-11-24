# Security Policy

## Reporting Security Issues

If you discover a security vulnerability in this project, please report it by emailing [your-email] or opening a private security advisory on GitHub.

**Please do not** open public issues for security vulnerabilities.

## Security Audit History

### November 2025 - Phase 2 Security Audit

**Status**: EXCELLENT ✅
- **Critical Issues**: 0
- **High Severity**: 0
- **Medium Severity**: 2
- **Low Severity**: 4

---

### Medium Severity Issues

#### 1. Default JWT Secret in Development
**Status**: IDENTIFIED - Fix pending
**Location**: `rust-backend/crates/shared/src/config.rs:126`

**Issue**: JWT secret has a development fallback value that could be used in production accidentally.

**Current Code**:
```rust
jwt_secret: env::var("JWT_SECRET")
    .unwrap_or_else(|_| "dev_secret_change_in_production".to_string()),
```

**Recommended Fix**: Make JWT_SECRET required in production:
```rust
jwt_secret: env::var("JWT_SECRET")
    .map_err(|_| Error::config("JWT_SECRET must be set"))?,
```

**Risk**: If deployed to production without setting `JWT_SECRET`, anyone can forge authentication tokens.

---

#### 2. CORS Configuration Too Permissive
**Status**: IDENTIFIED - Fix pending
**Location**: `rust-backend/crates/api-gateway/src/middleware.rs:9-15`

**Issue**: CORS middleware allows all HTTPS origins.

**Current Code**:
```rust
origin.as_bytes().starts_with(b"https://")
```

**Recommended Fix**: Whitelist specific origins from environment variable:
```rust
let allowed_origins = env::var("ALLOWED_ORIGINS")
    .unwrap_or_else(|_| "http://localhost:3000".to_string());

Cors::default()
    .allowed_origin_fn(move |origin, _req_head| {
        if cfg!(debug_assertions) {
            // Development: Allow localhost
            origin.as_bytes().starts_with(b"http://localhost")
        } else {
            // Production: Whitelist only
            allowed_origins.split(',').any(|o| origin.to_str().unwrap_or("") == o)
        }
    })
```

**Risk**: Any HTTPS website can make requests to your API, potentially enabling CSRF attacks.

---

### Low Severity Issues

#### 3. Unwrap() in Production Code
**Status**: IDENTIFIED
**Location**: `rust-backend/crates/api-gateway/src/handlers/health.rs:69`

Minor use of `.unwrap()` in health check serialization. Should use graceful error handling.

#### 4. Missing Rate Limiting on Health Endpoint
**Status**: IDENTIFIED

Health endpoint performs database query without rate limiting. Could be abused for DoS.

#### 5. No SQL Query Timeout
**Status**: IDENTIFIED

Database connection pool has no statement timeout configured. Long-running queries could tie up connections.

#### 6. TypeScript Type Safety
**Status**: IDENTIFIED
**Location**: `ponder-indexers/src/index.ts`

Event handlers use `any` type instead of proper Ponder-generated types. Should be fixed for type safety.

---

## Security Strengths

The following security best practices are already implemented:

### ✅ Environment Variable Security
- All sensitive credentials externalized to `.env`
- `.env.example` contains only placeholders
- `.gitignore` properly excludes `.env` files
- Clear security warnings in documentation

### ✅ Docker Security
- All services bind to `127.0.0.1` (localhost only)
- Strong password requirements enforced
- Resource limits prevent DoS attacks
- Pinned image versions (no `latest` tags)
- Health checks configured

### ✅ Database Security
- Parameterized queries only (SQLx compile-time verification)
- No raw SQL concatenation
- Foreign key constraints properly defined
- No SQL injection vulnerabilities found

### ✅ Dependency Security
- GitHub Actions security scanning (Trivy, Gitleaks)
- Weekly automated security audits
- Dependency audit on every PR
- Pinned versions in Cargo.toml and package.json

### ✅ Code Quality
- No `unsafe` blocks in production code
- Proper error handling with `thiserror` and `anyhow`
- Comprehensive input validation
- Structured logging (no sensitive data logged)

### ✅ Configuration Management
- Passwords required (not optional) for all services
- Clear separation of dev/staging/prod configs
- Strong password generation instructions

---

## Security Best Practices for Contributors

1. **Never commit secrets** - Use `.env` for all sensitive data
2. **Use parameterized queries** - Never concatenate SQL
3. **Avoid unwrap()** - Use proper error handling in production code
4. **Validate all inputs** - Use `validator` crate for Rust, Zod for TypeScript
5. **Keep dependencies updated** - Monitor security advisories
6. **Add tests** - Security-critical code must have tests
7. **Review OWASP Top 10** - Be aware of common vulnerabilities

---

## Compliance

This project follows security guidelines from:
- OWASP Top 10
- CWE/SANS Top 25
- Rust Security Guidelines
- Node.js Security Best Practices

---

## Update History

- **2025-11**: Initial security audit completed for Phase 2
- **2025-11**: Environment-based configuration implemented (commit fc7a4fb)
- **2025-11**: Docker security hardening completed
