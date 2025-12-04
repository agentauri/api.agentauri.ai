-- ============================================================================
-- Migration: Add Column-Level Encryption Functions (pgcrypto)
-- ============================================================================
-- Description: Adds encryption/decryption functions for protecting PII data
-- Created: 2025-11-30
--
-- SECURITY NOTES:
-- - Uses AES-256 symmetric encryption via pgcrypto
-- - Encryption key MUST be stored in a secure secrets manager (not in code!)
-- - For production, use AWS Secrets Manager, HashiCorp Vault, or similar
-- - Key rotation requires re-encrypting all data
-- ============================================================================

-- pgcrypto extension should already be enabled (migration 20250123000001)
-- This ensures it's available
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- ENCRYPTION FUNCTIONS
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Function: encrypt_text
-- Purpose: Encrypts plaintext using AES-256 symmetric encryption
-- Parameters:
--   plaintext TEXT - The text to encrypt
--   key TEXT       - The encryption key (from secrets manager)
-- Returns: Base64-encoded ciphertext
-- Usage: UPDATE users SET email = encrypt_text(email, 'key_from_vault');
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION encrypt_text(plaintext TEXT, key TEXT)
RETURNS TEXT AS $$
BEGIN
  -- pgp_sym_encrypt uses AES-256 with CFB mode
  -- encode to base64 for storage in TEXT column
  RETURN encode(
    pgp_sym_encrypt(
      plaintext::bytea,
      key,
      'cipher-algo=aes256, compress-algo=0'  -- No compression for PII
    ),
    'base64'
  );
EXCEPTION
  WHEN OTHERS THEN
    -- Log error but don't expose key in error message
    RAISE EXCEPTION 'Encryption failed: %', SQLERRM;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT;

COMMENT ON FUNCTION encrypt_text(TEXT, TEXT) IS
'Encrypts plaintext using AES-256. Key must come from secure secrets manager.';

-- ----------------------------------------------------------------------------
-- Function: decrypt_text
-- Purpose: Decrypts ciphertext encrypted with encrypt_text
-- Parameters:
--   ciphertext TEXT - The base64-encoded encrypted text
--   key TEXT        - The decryption key (must match encryption key)
-- Returns: Decrypted plaintext
-- Usage: SELECT decrypt_text(email, 'key_from_vault') FROM users;
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION decrypt_text(ciphertext TEXT, key TEXT)
RETURNS TEXT AS $$
BEGIN
  -- Decode from base64 and decrypt
  RETURN convert_from(
    pgp_sym_decrypt(
      decode(ciphertext, 'base64'),
      key
    ),
    'UTF8'
  );
EXCEPTION
  WHEN OTHERS THEN
    -- Return NULL instead of error for invalid/corrupted data
    -- This prevents application crashes from data corruption
    RETURN NULL;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT;

COMMENT ON FUNCTION decrypt_text(TEXT, TEXT) IS
'Decrypts ciphertext encrypted with encrypt_text. Returns NULL on error.';

-- ============================================================================
-- ENCRYPTION KEY MANAGEMENT TABLE
-- ============================================================================
-- Stores metadata about encryption keys (NOT the keys themselves!)
-- Actual keys are stored in external secrets manager

CREATE TABLE IF NOT EXISTS encryption_keys (
  id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
  key_name TEXT NOT NULL UNIQUE,           -- e.g., 'pii_encryption_key_v1'
  key_version INTEGER NOT NULL DEFAULT 1,  -- For key rotation
  algorithm TEXT NOT NULL DEFAULT 'AES256',
  created_at TIMESTAMPTZ DEFAULT NOW(),
  rotated_at TIMESTAMPTZ,                  -- When key was rotated
  deprecated_at TIMESTAMPTZ,               -- When key was deprecated
  external_key_id TEXT,                    -- Reference to secrets manager
  is_active BOOLEAN DEFAULT true,

  CONSTRAINT chk_key_version_positive CHECK (key_version > 0)
);

CREATE INDEX idx_encryption_keys_active ON encryption_keys(is_active) WHERE is_active = true;
CREATE INDEX idx_encryption_keys_name_version ON encryption_keys(key_name, key_version);

COMMENT ON TABLE encryption_keys IS
'Metadata for encryption keys (keys themselves stored in secrets manager)';

-- Insert default key metadata (key stored externally)
INSERT INTO encryption_keys (key_name, key_version, algorithm, external_key_id)
VALUES ('pii_encryption_key', 1, 'AES256', 'SECRET_MANAGER_KEY_ID')
ON CONFLICT (key_name) DO NOTHING;

-- ============================================================================
-- AUDIT LOGGING FOR ENCRYPTED DATA ACCESS
-- ============================================================================
-- Tracks when encrypted data is accessed (for compliance)

-- Note: Using accessed_at as part of composite key for TimescaleDB hypertable compatibility
-- TimescaleDB requires partitioning column in unique indexes
CREATE TABLE IF NOT EXISTS encrypted_data_access_log (
  id BIGSERIAL NOT NULL,
  table_name TEXT NOT NULL,
  column_name TEXT NOT NULL,
  row_id TEXT NOT NULL,                    -- ID of accessed row
  accessed_by TEXT NOT NULL,               -- User who accessed data
  access_type TEXT NOT NULL,               -- 'read', 'write', 'delete'
  accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  client_ip INET,

  CONSTRAINT chk_access_type CHECK (access_type IN ('read', 'write', 'delete'))
);

-- Convert to hypertable BEFORE creating indexes
-- This allows TimescaleDB to manage indexes properly
SELECT create_hypertable(
  'encrypted_data_access_log',
  'accessed_at',
  chunk_time_interval => INTERVAL '7 days',
  if_not_exists => TRUE
);

-- Create indexes after hypertable conversion
CREATE INDEX idx_encrypted_access_table_row ON encrypted_data_access_log(table_name, row_id, accessed_at);
CREATE INDEX idx_encrypted_access_user ON encrypted_data_access_log(accessed_by, accessed_at);
CREATE INDEX idx_encrypted_access_time ON encrypted_data_access_log(accessed_at DESC);

COMMENT ON TABLE encrypted_data_access_log IS
'Audit log for access to encrypted data (GDPR/HIPAA compliance)';

-- ============================================================================
-- HELPER FUNCTIONS FOR ENCRYPTED COLUMNS
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Function: log_encrypted_access
-- Purpose: Logs access to encrypted data
-- Parameters:
--   p_table_name TEXT   - Table containing encrypted data
--   p_column_name TEXT  - Column that was accessed
--   p_row_id TEXT       - ID of the row
--   p_user TEXT         - User accessing the data
--   p_access_type TEXT  - 'read', 'write', or 'delete'
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION log_encrypted_access(
  p_table_name TEXT,
  p_column_name TEXT,
  p_row_id TEXT,
  p_user TEXT,
  p_access_type TEXT
)
RETURNS VOID AS $$
BEGIN
  INSERT INTO encrypted_data_access_log (
    table_name,
    column_name,
    row_id,
    accessed_by,
    access_type,
    client_ip
  ) VALUES (
    p_table_name,
    p_column_name,
    p_row_id,
    p_user,
    p_access_type,
    inet_client_addr()  -- Capture client IP
  );
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION log_encrypted_access IS
'Logs access to encrypted data for audit trail';

-- ============================================================================
-- EXAMPLE: Encrypting Existing Columns (COMMENTED OUT)
-- ============================================================================
-- IMPORTANT: These are examples. Run manually after backing up data!
-- DO NOT run automatically in migration!

-- Example 1: Encrypt email addresses in users table
-- -------------------------------------------------------
-- Step 1: Add new encrypted column
-- ALTER TABLE users ADD COLUMN email_encrypted TEXT;
--
-- Step 2: Encrypt existing data (requires encryption key from secrets manager)
-- UPDATE users
-- SET email_encrypted = encrypt_text(email, 'YOUR_KEY_FROM_SECRETS_MANAGER')
-- WHERE email IS NOT NULL;
--
-- Step 3: Drop old plaintext column
-- ALTER TABLE users DROP COLUMN email;
--
-- Step 4: Rename encrypted column
-- ALTER TABLE users RENAME COLUMN email_encrypted TO email;
--
-- Step 5: Add constraint to ensure encryption
-- ALTER TABLE users ADD CONSTRAINT chk_email_encrypted
--   CHECK (email IS NULL OR length(email) > 50);  -- Encrypted data is longer

-- Example 2: Encrypt API key hashes (additional layer)
-- -------------------------------------------------------
-- Note: API keys are already hashed with Argon2id
-- This adds encryption on top of hashing for defense-in-depth
--
-- ALTER TABLE api_keys ADD COLUMN key_hash_encrypted TEXT;
-- UPDATE api_keys
-- SET key_hash_encrypted = encrypt_text(key_hash, 'YOUR_KEY_FROM_SECRETS_MANAGER')
-- WHERE key_hash IS NOT NULL;
-- ALTER TABLE api_keys DROP COLUMN key_hash;
-- ALTER TABLE api_keys RENAME COLUMN key_hash_encrypted TO key_hash;

-- ============================================================================
-- PERFORMANCE CONSIDERATIONS
-- ============================================================================
--
-- Encryption overhead:
-- - Encryption: ~0.5ms per operation (varies with data size)
-- - Decryption: ~0.5ms per operation
-- - Storage: ~33% increase (base64 encoding + encryption overhead)
--
-- Recommendations:
-- 1. Only encrypt PII/sensitive data (email, phone, SSN, etc.)
-- 2. Use database connection pooling to amortize decryption cost
-- 3. Cache decrypted data in application layer when appropriate
-- 4. Use partial indexes for encrypted columns
-- 5. Consider application-layer encryption for extremely sensitive data
--
-- ============================================================================

-- Grant execute permissions to application role
-- (Adjust role name based on your setup)
GRANT EXECUTE ON FUNCTION encrypt_text(TEXT, TEXT) TO postgres;
GRANT EXECUTE ON FUNCTION decrypt_text(TEXT, TEXT) TO postgres;
GRANT EXECUTE ON FUNCTION log_encrypted_access(TEXT, TEXT, TEXT, TEXT, TEXT) TO postgres;
