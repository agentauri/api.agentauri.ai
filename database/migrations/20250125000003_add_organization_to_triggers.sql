-- Migration: Add organization_id to triggers table
-- Description: Link triggers to organizations for multi-tenant support
-- Phase: 3.5 Week 11 - Account Model + Organizations
-- Note: Column is nullable initially to allow data migration in the next migration

-- Add organization_id column (nullable for migration compatibility)
ALTER TABLE triggers ADD COLUMN organization_id TEXT;

-- Add foreign key constraint
ALTER TABLE triggers ADD CONSTRAINT fk_organization
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;

-- Create partial index for finding triggers by organization (only when organization_id is set)
CREATE INDEX idx_triggers_organization_id ON triggers(organization_id)
    WHERE organization_id IS NOT NULL;

COMMENT ON COLUMN triggers.organization_id IS 'Organization that owns this trigger (nullable until data migration)';
