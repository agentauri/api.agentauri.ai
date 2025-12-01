# Security Headers - Quick Reference Card

## 🎯 Quick Start

```rust
// In main.rs (already configured)
App::new()
    .wrap(SecurityHeaders::for_api())  // ✅ Use this for APIs
    // ... routes
```

## 📋 Headers Checklist (10 total)

| # | Header | Value | ✓ |
|---|--------|-------|---|
| 1 | X-Content-Type-Options | `nosniff` | ✅ |
| 2 | X-Frame-Options | `DENY` | ✅ |
| 3 | X-XSS-Protection | `1; mode=block` | ✅ |
| 4 | Referrer-Policy | `strict-origin-when-cross-origin` | ✅ |
| 5 | Permissions-Policy | `geolocation=(), camera=()...` | ✅ |
| 6 | Cross-Origin-Embedder-Policy | `require-corp` | ✅ |
| 7 | Cross-Origin-Opener-Policy | `same-origin` | ✅ |
| 8 | Cross-Origin-Resource-Policy | `same-origin` | ✅ |
| 9 | Strict-Transport-Security | `max-age=31536000...` | ⚠️ Production only |
| 10 | Content-Security-Policy | `default-src 'self'` | ⚠️ Disabled for APIs |

## 🧪 Testing Commands

```bash
# Unit tests (12 tests)
cargo test --package api-gateway --lib middleware::security_headers

# Integration tests (13 tests)
cargo test --package api-gateway --test security_headers_integration_test

# Manual test (local)
./scripts/test-security-headers.sh

# Manual test (production)
./scripts/test-security-headers.sh https://api.8004.dev
```

## 🔧 Configuration

### Development
```bash
# .env
ENABLE_HSTS=false  # HTTP testing
```

### Production
```bash
# .env
ENABLE_HSTS=true   # Enforce HTTPS
HSTS_MAX_AGE=31536000
```

## 🐛 Common Issues

### Issue: Grade is A instead of A+
**Fix**: Enable HSTS in production
```bash
ENABLE_HSTS=true
```

### Issue: Headers missing
**Fix**: Verify middleware is applied
```rust
.wrap(SecurityHeaders::for_api())
```

### Issue: CSP blocking API
**Fix**: Use API config (no CSP)
```rust
SecurityHeaders::for_api()  // ✅ Correct
SecurityHeaders::default()  // ❌ Wrong for APIs
```

## 📊 Expected Grades

| Environment | HSTS | Grade |
|-------------|------|-------|
| Development (HTTP) | ❌ | A |
| Production (HTTPS) | ✅ | **A+** |

## 🔍 Verify Headers

```bash
# Quick check
curl -I https://api.8004.dev/api/v1/health | grep -i "strict-transport-security\|cross-origin"

# Expected:
# Strict-Transport-Security: max-age=31536000; includeSubDomains
# Cross-Origin-Embedder-Policy: require-corp
# Cross-Origin-Opener-Policy: same-origin
# Cross-Origin-Resource-Policy: same-origin
```

## 📚 Documentation

- **Complete Guide**: [SECURITY_HEADERS.md](./SECURITY_HEADERS.md)
- **Validation**: [SECURITYHEADERS_COM_VALIDATION.md](./SECURITYHEADERS_COM_VALIDATION.md)
- **Implementation**: [SECURITY_HEADERS_IMPLEMENTATION.md](../../rust-backend/crates/api-gateway/SECURITY_HEADERS_IMPLEMENTATION.md)

## 🎖️ SecurityHeaders.com Validation

1. Visit: https://securityheaders.com
2. Enter: `https://api.8004.dev/api/v1/health`
3. Expected: **A+**

## 📞 Support

**Issues?**
1. Check logs: `journalctl -u api-gateway -n 100`
2. Run tests: `./scripts/test-security-headers.sh`
3. Review config: `grep SecurityHeaders src/main.rs`

---

**Last Updated**: 2025-11-30
**Status**: ✅ Production Ready
**Grade**: A+
