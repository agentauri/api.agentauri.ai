-- Migration: Add Account Lockout Fields to Users
-- Description: Implements brute-force protection via account lockout
-- Created: 2025-12-02
-- Phase: Production Hardening

-- Add account lockout fields to users table
-- These fields track failed login attempts and implement progressive lockout
ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN locked_until TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN last_failed_login TIMESTAMPTZ;

-- Index for finding locked accounts (partial index for efficiency)
-- Only indexes rows where locked_until is set
CREATE INDEX idx_users_locked_until ON users(locked_until)
    WHERE locked_until IS NOT NULL;

-- Comments explaining lockout policy
COMMENT ON COLUMN users.failed_login_attempts IS 'Count of consecutive failed login attempts. Resets on successful login.';
COMMENT ON COLUMN users.locked_until IS 'Account locked until this timestamp. NULL means not locked. Progressive: 15min, 30min, 1h, 2h, 4h.';
COMMENT ON COLUMN users.last_failed_login IS 'Timestamp of last failed login attempt for audit purposes.';

-- Note: Lockout policy implemented in application code (handlers/auth.rs):
-- - Threshold: 5 failed attempts
-- - Lockout duration: Progressive (15min base, doubles up to 4h max)
-- - Reset: Successful login resets counter and unlocks account
