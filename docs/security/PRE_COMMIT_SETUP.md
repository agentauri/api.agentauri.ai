# Pre-Commit Hook Setup for Secret Detection

This guide helps you set up automated secret detection to prevent hardcoded credentials from being committed to the repository.

## Quick Setup

### 1. Install Gitleaks

**macOS:**
```bash
brew install gitleaks
```

**Ubuntu/Debian:**
```bash
# Download latest release
wget https://github.com/gitleaks/gitleaks/releases/download/v8.18.1/gitleaks_8.18.1_linux_x64.tar.gz
tar -xzf gitleaks_8.18.1_linux_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
```

**Windows:**
```powershell
# Using Scoop
scoop install gitleaks

# Or download from GitHub releases
# https://github.com/gitleaks/gitleaks/releases
```

### 2. Install Pre-Commit Hook

Create the pre-commit hook file:

```bash
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Pre-commit hook to detect secrets using gitleaks

echo "🔍 Scanning for secrets with gitleaks..."

# Run gitleaks on staged files
gitleaks protect --staged --verbose

if [ $? -eq 1 ]; then
    echo ""
    echo "❌ COMMIT REJECTED: Gitleaks detected secrets in your staged files!"
    echo ""
    echo "What to do:"
    echo "1. Review the output above to identify the exposed secret"
    echo "2. Remove the hardcoded credential from your code"
    echo "3. Use environment variables instead (see database/README.md)"
    echo "4. If this is a false positive, add it to .gitleaksignore"
    echo ""
    exit 1
fi

echo "✅ No secrets detected"
exit 0
EOF

# Make the hook executable
chmod +x .git/hooks/pre-commit
```

### 3. Configure Gitleaks (Optional)

Create `.gitleaks.toml` in project root for custom rules:

```toml
# Custom gitleaks configuration
title = "AgentAuri Backend Secret Detection"

# Custom rules for database URLs
[[rules]]
id = "postgres-url-with-password"
description = "PostgreSQL connection string with embedded password"
regex = '''postgresql://[^:]+:[^@]+@'''
tags = ["database", "credentials"]

[[rules]]
id = "redis-url-with-password"
description = "Redis connection string with embedded password"
regex = '''redis://:[^@]+@'''
tags = ["database", "credentials"]

# Allow example files
[allowlist]
paths = [
    '''.env.example''',
    '''.env.test.example''',
    '''docs/.*\.md''',  # Documentation may contain example URLs
]

# Allow common test patterns
regexes = [
    '''YOUR_PASSWORD''',
    '''password@localhost''',
    '''postgres@localhost''',
    '''user:pass@localhost''',  # Generic test examples in docs
]
```

### 4. Test the Hook

Try committing a file with a hardcoded credential:

```bash
# Create a test file with a fake secret
echo 'DATABASE_URL="postgresql://user:secret123@localhost/db"' > test_secret.txt

# Try to stage and commit
git add test_secret.txt
git commit -m "test"

# Should be rejected by pre-commit hook!
# Clean up
git reset HEAD test_secret.txt
rm test_secret.txt
```

## Alternative: Pre-Commit Framework

For more advanced hook management, use the [pre-commit](https://pre-commit.com/) framework:

### 1. Install Pre-Commit

```bash
pip install pre-commit
# or
brew install pre-commit
```

### 2. Create `.pre-commit-config.yaml`

```yaml
repos:
  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.18.1
    hooks:
      - id: gitleaks

  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: check-added-large-files
      - id: check-yaml
      - id: end-of-file-fixer
      - id: trailing-whitespace
      - id: detect-private-key

  - repo: local
    hooks:
      - id: rustfmt
        name: rustfmt
        entry: cargo fmt --all --
        language: system
        types: [rust]
        pass_filenames: false

      - id: clippy
        name: clippy
        entry: cargo clippy --all-targets --all-features -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
```

### 3. Install Hooks

```bash
pre-commit install
```

### 4. Run Manually (Optional)

```bash
# Run on all files
pre-commit run --all-files

# Run on staged files only
pre-commit run
```

## CI Integration

Add Gitleaks to your CI pipeline (GitHub Actions example):

```yaml
# .github/workflows/security.yml
name: Security Scan

on: [push, pull_request]

jobs:
  gitleaks:
    name: Secret Scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run Gitleaks
        uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## False Positives

If gitleaks reports false positives:

### Option 1: Add to `.gitleaksignore`

```bash
# .gitleaksignore
# File-level ignores
docs/examples/connection_string.md:5

# Pattern ignores (use carefully!)
**/test_data.sql:*
```

### Option 2: Inline Comments

```rust
// gitleaks:allow
let example_url = "postgresql://user:password@localhost/db";  // Example only
```

### Option 3: Update Configuration

Edit `.gitleaks.toml` to refine rules or add allowlist patterns.

## Best Practices

1. **Never disable the hook** for convenience - it's your safety net
2. **Review alerts carefully** - false positives are rare, usually indicate real issues
3. **Use environment variables** for all credentials (see `database/README.md`)
4. **Keep `.env` files in `.gitignore`** (already configured)
5. **Rotate any credentials** that were accidentally committed (see git history)
6. **Train your team** on secure credential management

## Git History Scanning

To scan existing git history for leaked secrets:

```bash
# Scan entire repository history
gitleaks detect --verbose

# Scan specific branch
gitleaks detect --source . --log-opts="origin/main"

# Generate report
gitleaks detect --report-path gitleaks-report.json
```

If secrets are found in history:

1. **Rotate the exposed credentials immediately**
2. **Consider using BFG Repo-Cleaner** to remove from history:
   ```bash
   # DANGEROUS: Only use on feature branches, never on shared branches
   git filter-repo --path-match '*.env' --invert-paths
   ```
3. **Force push** is required (coordinate with team!)
4. **All team members** must re-clone the repository

## Troubleshooting

### Hook Not Running

```bash
# Verify hook is executable
ls -la .git/hooks/pre-commit

# If not executable:
chmod +x .git/hooks/pre-commit
```

### Gitleaks Not Found

```bash
# Check if gitleaks is in PATH
which gitleaks

# Install if missing (see installation section above)
```

### Want to Bypass Hook (NOT RECOMMENDED)

```bash
# Emergency only - requires explanation in commit message
git commit --no-verify -m "Emergency fix (hook bypassed: reason)"

# Better: Fix the issue properly!
```

## Additional Resources

- [Gitleaks Documentation](https://github.com/gitleaks/gitleaks)
- [Pre-Commit Framework](https://pre-commit.com/)
- [OWASP Secrets Management](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [GitHub Secret Scanning](https://docs.github.com/en/code-security/secret-scanning)

## Support

If you encounter issues with the pre-commit hook:

1. Check this documentation first
2. Ask in the team security channel
3. Review gitleaks logs: `gitleaks detect --verbose`
4. Open an issue with the security team

---

**Remember**: The pre-commit hook is your first line of defense against credential leaks. Keep it enabled and functioning!

**Last Updated**: 2025-11-29
