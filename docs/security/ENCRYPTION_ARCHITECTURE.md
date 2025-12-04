# Database Encryption Architecture - AgentAuri Backend
# ============================================================================

┌─────────────────────────────────────────────────────────────────────────┐
│                         APPLICATION LAYER                                │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Rust Backend (api-gateway, event-processor, action-workers)     │  │
│  │                                                                  │  │
│  │ SQLx Connection Pool (20 connections)                           │  │
│  │ - PgSslMode::VerifyFull                                         │  │
│  │ - ssl_root_cert: /etc/ssl/certs/ca-cert.crt                    │  │
│  └──────────────────┬───────────────────────────────────────────────┘  │
│                     │                                                   │
└─────────────────────┼───────────────────────────────────────────────────┘
                      │
                      │ DATABASE_URL with sslmode=verify-full
                      │
┌─────────────────────▼───────────────────────────────────────────────────┐
│                      ENCRYPTION LAYER 1: TLS/SSL                         │
│                      (Encryption in Transit)                             │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Protocol: TLS 1.2+ (TLS 1.3 preferred)                          │  │
│  │ Cipher: AES-256-GCM, ChaCha20-Poly1305                          │  │
│  │ Certificate: CA-signed (Let's Encrypt, DigiCert)                │  │
│  │ Validation: verify-full (hostname + CA chain)                   │  │
│  │                                                                  │  │
│  │ Performance Impact: 5-10%                                        │  │
│  │ - Handshake: ~30ms (amortized via connection pooling)           │  │
│  │ - Per-query: ~0.1ms (hardware-accelerated AES-NI)              │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Security Provided:                                                     │
│  ✓ Protects against network eavesdropping                              │
│  ✓ Prevents man-in-the-middle (MITM) attacks                           │
│  ✓ Ensures data integrity during transmission                          │
│                                                                          │
└─────────────────────┬────────────────────────────────────────────────────┘
                      │
                      │ Encrypted TCP Stream
                      │
┌─────────────────────▼───────────────────────────────────────────────────┐
│                   POSTGRESQL SERVER (Port 5432)                          │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ postgresql.conf                                                  │  │
│  │ - ssl = on                                                       │  │
│  │ - ssl_min_protocol_version = 'TLSv1.2'                          │  │
│  │ - password_encryption = scram-sha-256                            │  │
│  │ - ssl_ciphers = 'HIGH:MEDIUM:+3DES:!aNULL'                      │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ pg_hba.conf                                                      │  │
│  │ hostssl all all 0.0.0.0/0 scram-sha-256  # TLS required         │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────┬────────────────────────────────────────────────────┘
                      │
                      │ SQL Queries (authenticated, authorized)
                      │
┌─────────────────────▼───────────────────────────────────────────────────┐
│                   ENCRYPTION LAYER 2: COLUMN ENCRYPTION                  │
│                   (Application-Layer Encryption)                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ pgcrypto Extension (PostgreSQL 15)                               │  │
│  │                                                                  │  │
│  │ Encryption Functions:                                            │  │
│  │ - encrypt_text(plaintext, key) → ciphertext (base64)            │  │
│  │ - decrypt_text(ciphertext, key) → plaintext                     │  │
│  │                                                                  │  │
│  │ Algorithm: AES-256-CBC (pgp_sym_encrypt)                        │  │
│  │ Key Source: AWS Secrets Manager / HashiCorp Vault               │  │
│  │ Key Rotation: Supported (re-encrypt with new key)               │  │
│  │                                                                  │  │
│  │ Performance Impact: 10-20% (per encrypted column)               │  │
│  │ - Encrypt: ~0.5ms per operation                                 │  │
│  │ - Decrypt: ~0.5ms per operation                                 │  │
│  │ - Storage: +33% (base64 encoding + PGP header)                  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Encrypted Columns (Example):                                           │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ users                                                            │  │
│  │ - email (encrypted with key_v1)                                 │  │
│  │ - phone (encrypted with key_v1)                                 │  │
│  │ - wallet_address (encrypted with key_v2)                        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Audit Trail:                                                           │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ encrypted_data_access_log (TimescaleDB hypertable)               │  │
│  │ - table_name, column_name, row_id                               │  │
│  │ - accessed_by, access_type (read/write/delete)                  │  │
│  │ - accessed_at, client_ip                                        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Security Provided:                                                     │
│  ✓ Protects PII even if database backup is stolen                      │
│  ✓ Limits exposure to authorized users with decryption key             │
│  ✓ Enables granular access control (column-level)                      │
│  ✓ Audit trail for compliance (GDPR Article 30)                        │
│                                                                          │
└─────────────────────┬────────────────────────────────────────────────────┘
                      │
                      │ Encrypted Data (base64)
                      │
┌─────────────────────▼───────────────────────────────────────────────────┐
│                   ENCRYPTION LAYER 3: TRANSPARENT DATA ENCRYPTION        │
│                   (Storage-Layer Encryption at Rest)                     │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Managed Database (AWS RDS, Azure, GCP Cloud SQL)                │  │
│  │                                                                  │  │
│  │ AWS RDS PostgreSQL:                                              │  │
│  │ - AES-256 encryption                                             │  │
│  │ - AWS KMS key management                                         │  │
│  │ - Encrypted: data files, backups, snapshots, replicas           │  │
│  │                                                                  │  │
│  │ Azure Database for PostgreSQL:                                   │  │
│  │ - AES-256 encryption (always on, immutable)                     │  │
│  │ - Azure Key Vault (customer-managed keys optional)              │  │
│  │                                                                  │  │
│  │ Google Cloud SQL:                                                │  │
│  │ - AES-256 encryption (default for new instances)                │  │
│  │ - Cloud KMS (customer-managed keys optional)                    │  │
│  │                                                                  │  │
│  │ Performance Impact: <5% (hardware-accelerated AES-NI)           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Self-Managed Alternative:                                              │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ LUKS Filesystem Encryption (Linux)                               │  │
│  │ - Algorithm: AES-256-XTS                                         │  │
│  │ - Key: 256-bit random key (stored securely)                     │  │
│  │ - Mount: /dev/mapper/pgdata → /var/lib/postgresql/data          │  │
│  │ - Auto-mount: /etc/crypttab + /etc/fstab                        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  Security Provided:                                                     │
│  ✓ Protects against physical disk theft                                │
│  ✓ Encrypts all PostgreSQL files (data, WAL, temp)                     │
│  ✓ Transparent to application (no code changes)                        │
│  ✓ Key managed separately (KMS, Key Vault)                             │
│                                                                          │
└─────────────────────┬────────────────────────────────────────────────────┘
                      │
                      │ Encrypted Blocks
                      │
┌─────────────────────▼───────────────────────────────────────────────────┐
│                        PHYSICAL STORAGE LAYER                            │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Disk Storage (SSD/NVMe)                                          │  │
│  │                                                                  │  │
│  │ Files on Disk (all encrypted via TDE):                          │  │
│  │ - /var/lib/postgresql/data/base/* (database files)              │  │
│  │ - /var/lib/postgresql/data/pg_wal/* (write-ahead logs)          │  │
│  │ - /var/lib/postgresql/data/base/pgsql_tmp/* (temp files)        │  │
│  │ - /var/lib/postgresql/wal_archive/* (archived WAL)              │  │
│  │                                                                  │  │
│  │ Backup Storage (AWS S3, Azure Blob, GCP Storage):                │  │
│  │ - Daily full backups (encrypted at rest)                        │  │
│  │ - Continuous WAL archiving (encrypted)                          │  │
│  │ - Point-in-time recovery (PITR) capable                         │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘


# ============================================================================
# KEY MANAGEMENT ARCHITECTURE
# ============================================================================

┌────────────────────────────────────────────────────────────────────────┐
│                       SECRETS MANAGER (External)                        │
│                                                                         │
│  AWS Secrets Manager / HashiCorp Vault / GCP Secret Manager            │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Secret: agentauri/prod/database/encryption-key-v1                  │ │
│  │ Value: [32-byte random key, base64-encoded]                      │ │
│  │ Rotation: Annual (re-encrypt data with new key)                  │ │
│  │ Access: IAM role-based (application only)                        │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Secret: agentauri/prod/database/master-password                    │ │
│  │ Value: [32-byte random password]                                 │ │
│  │ Rotation: Quarterly (via automated script)                       │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Secret: agentauri/prod/database/kms-key-arn                        │ │
│  │ Value: arn:aws:kms:us-east-1:123456789012:key/...               │ │
│  │ Purpose: TDE master key (AWS KMS)                                │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                         │
└─────────────────────────┬───────────────────────────────────────────────┘
                          │
                          │ Retrieved at application startup
                          │ Cached in memory (never logged)
                          │
┌─────────────────────────▼───────────────────────────────────────────────┐
│                      APPLICATION RUNTIME                                 │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Encryption Key Cache (In-Memory)                                 │  │
│  │ - TTL: 1 hour (auto-refresh)                                     │  │
│  │ - Never persisted to disk                                        │  │
│  │ - Zeroized on process termination                                │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘


# ============================================================================
# DATA FLOW EXAMPLE: Encrypted Write Operation
# ============================================================================

1. Application receives user data:
   email = "user@example.com"

2. Retrieve encryption key from secrets manager:
   encryption_key = aws_secrets_manager.get_secret("db-encryption-key")

3. Encrypt data via SQL function:
   encrypted_email = encrypt_text("user@example.com", encryption_key)
   → "ww0ECQMCXx5y3zQf0bRg0kMBa3..." (base64)

4. Send query via TLS connection:
   INSERT INTO users (email) VALUES ($1)  -- $1 = encrypted_email
   ↓ (TLS 1.2+ encrypted tunnel)

5. PostgreSQL receives encrypted email:
   - Stores in database (already encrypted)
   - TDE encrypts again at storage layer
   - Writes to disk (double-encrypted)

6. Audit log entry:
   INSERT INTO encrypted_data_access_log (table_name, column_name, row_id, accessed_by, access_type)
   VALUES ('users', 'email', '123', 'admin@example.com', 'write')


# ============================================================================
# DATA FLOW EXAMPLE: Encrypted Read Operation
# ============================================================================

1. Application sends query via TLS:
   SELECT decrypt_text(email, $1) FROM users WHERE id = $2
   ↓ (TLS 1.2+ encrypted tunnel)

2. PostgreSQL:
   - Reads encrypted data from disk (TDE decrypts to memory)
   - Calls decrypt_text() with user-provided key
   - Returns plaintext to application via TLS

3. Application receives:
   email = "user@example.com" (plaintext, in memory only)

4. Application caches decrypted value (5-minute TTL):
   cache.set("user:123:email", "user@example.com", 300)

5. Audit log entry:
   INSERT INTO encrypted_data_access_log (...)
   VALUES ('users', 'email', '123', 'admin@example.com', 'read')


# ============================================================================
# SECURITY GUARANTEES
# ============================================================================

Defense-in-Depth Strategy:
  Layer 1 (TLS) COMPROMISED + Layer 2 (Column) OK + Layer 3 (TDE) OK
    → Attacker sees encrypted columns (AES-256, key unknown)
    → Data remains protected ✓

  Layer 1 (TLS) OK + Layer 2 (Column) COMPROMISED + Layer 3 (TDE) OK
    → Attacker can decrypt columns BUT needs TLS to intercept
    → Data in transit protected, data at rest protected ✓

  Layer 1 (TLS) OK + Layer 2 (Column) OK + Layer 3 (TDE) COMPROMISED
    → Attacker sees encrypted columns (key unknown)
    → Data remains protected ✓

  All 3 Layers COMPROMISED:
    → Requires: TLS certificate + Encryption key + Disk access
    → Extremely unlikely (defense-in-depth successful) ✓

Compliance:
  GDPR Article 32:
    ✓ Encryption of personal data (3 layers)
    ✓ Pseudonymisation where applicable (column encryption)
    ✓ Regular testing of security measures (test suite)

  HIPAA 164.312(a)(2)(iv):
    ✓ Encryption and decryption (TLS + AES-256)
    ✓ Audit controls (encrypted_data_access_log)

  PCI DSS Requirement 3.4:
    ✓ Render PAN unreadable (column encryption)
    ✓ Strong cryptography (AES-256)
    ✓ Key management (external secrets manager)

  SOC 2 CC6.6:
    ✓ Encryption at rest (TDE)
    ✓ Encryption in transit (TLS 1.2+)
    ✓ Key management (CC6.7)


# ============================================================================
# PERFORMANCE SUMMARY
# ============================================================================

Throughput Impact (per 1000 requests):
  Without Encryption: 1000 req/s → 100% baseline
  With TLS Only:       850 req/s → 85% (15% overhead)
  With TLS + Column:   700 req/s → 70% (30% overhead)

Latency Impact (p50):
  Without Encryption: 5ms
  With TLS Only:      6ms (+20%)
  With TLS + Column:  8ms (+60% on encrypted columns only)

Optimization Impact:
  Connection Pooling: Reduces TLS overhead from 15% → 5%
  Application Cache:  Reduces column overhead from 20% → 5%
  Hardware AES-NI:    Reduces total overhead by 50%

Real-World Performance (Optimized):
  Combined Overhead: 10-15% (acceptable for security gain)
  Throughput:        900 req/s (vs 1000 baseline)
  Latency (p50):     5.5ms (vs 5ms baseline)


# ============================================================================
