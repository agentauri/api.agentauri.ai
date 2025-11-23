-- Migration 2: Create Helper Functions
-- Description: Creates reusable database functions for common operations
-- Created: 2025-01-23

-- Function: update_updated_at_column()
-- Purpose: Automatically updates the updated_at timestamp column on row updates
-- Usage: Attach as BEFORE UPDATE trigger to any table with updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
