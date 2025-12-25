-- Migration: Add display_name to users table
-- Description: Adds a human-friendly display name field for user profiles
-- Created: 2025-12-25

-- Add display_name column (nullable, used for profile display)
ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name TEXT;

-- Comment for documentation
COMMENT ON COLUMN users.display_name IS 'Human-friendly display name, distinct from unique username. Populated from OAuth provider or user-set.';

-- Index for potential future search functionality
CREATE INDEX IF NOT EXISTS idx_users_display_name ON users(display_name) WHERE display_name IS NOT NULL;
