-- Migration: Add request_uri field for ValidationRequest events
-- Date: 2025-12-01
-- Issue: ValidationRequest handler tries to write to non-existent request_uri field

-- Add the missing column
ALTER TABLE events ADD COLUMN request_uri TEXT;

-- Add index for request_uri queries (partial index for non-null values)
CREATE INDEX idx_events_request_uri
  ON events(request_uri)
  WHERE request_uri IS NOT NULL;

-- Add comment for documentation
COMMENT ON COLUMN events.request_uri IS
  'URI containing validation request data (ValidationRequest event only)';
