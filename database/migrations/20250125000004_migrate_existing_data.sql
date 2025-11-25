-- Migration: Migrate existing data to organization model
-- Description: Create personal organizations for existing users and migrate their triggers
-- Phase: 3.5 Week 11 - Account Model + Organizations
-- Warning: This migration modifies existing data

-- Step 1: Create personal organizations for all existing users who don't have one
INSERT INTO organizations (id, name, slug, description, owner_id, is_personal, created_at, updated_at)
SELECT
    gen_random_uuid()::TEXT,
    username || '''s Workspace',
    LOWER(username) || '-' || SUBSTRING(id FROM 1 FOR 8),
    'Personal workspace',
    id,
    true,
    NOW(),
    NOW()
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM organizations o
    WHERE o.owner_id = u.id AND o.is_personal = true
);

-- Step 2: Add owners as members with 'owner' role
INSERT INTO organization_members (id, organization_id, user_id, role, created_at)
SELECT
    gen_random_uuid()::TEXT,
    o.id,
    o.owner_id,
    'owner',
    NOW()
FROM organizations o
WHERE NOT EXISTS (
    SELECT 1 FROM organization_members m
    WHERE m.organization_id = o.id AND m.user_id = o.owner_id
);

-- Step 3: Migrate all triggers to their owner's personal organization
UPDATE triggers t
SET organization_id = (
    SELECT o.id FROM organizations o
    WHERE o.owner_id = t.user_id AND o.is_personal = true
    LIMIT 1
)
WHERE t.organization_id IS NULL;

-- Step 4: Make organization_id NOT NULL now that all triggers have been migrated
ALTER TABLE triggers ALTER COLUMN organization_id SET NOT NULL;

-- Step 5: Drop the old partial index and create a new one that includes organization_id
DROP INDEX IF EXISTS idx_triggers_chain_registry_enabled;

-- Create optimized composite index for event processor queries
-- This index supports: WHERE organization_id = ? AND chain_id = ? AND registry = ? AND enabled = true
CREATE INDEX idx_triggers_org_chain_registry_enabled
    ON triggers(organization_id, chain_id, registry)
    WHERE enabled = true;

-- Update comment on organization_id column
COMMENT ON COLUMN triggers.organization_id IS 'Organization that owns this trigger (required)';
