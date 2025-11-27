-- Migration: Add Wallet Address to Users
-- Description: Enable wallet-based authentication (Layer 2)
-- Created: 2025-11-27

-- Add wallet_address column to users table
-- This enables EIP-191 signature-based authentication
ALTER TABLE users ADD COLUMN wallet_address TEXT UNIQUE;

-- Index for fast wallet lookups (only for non-null values)
CREATE INDEX idx_users_wallet_address ON users(wallet_address) WHERE wallet_address IS NOT NULL;

-- Comment on column
COMMENT ON COLUMN users.wallet_address IS 'Ethereum wallet address linked to user (lowercase, checksummed)';
