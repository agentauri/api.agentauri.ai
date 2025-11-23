-- Migration 3: Create users Table
-- Description: Stores user accounts for API authentication
-- Created: 2025-01-23

CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL, -- bcrypt hash
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true
);

-- Index on email for fast lookups during authentication
CREATE INDEX idx_users_email ON users(email);

-- Index on username for fast lookups and uniqueness checks
CREATE INDEX idx_users_username ON users(username);

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
