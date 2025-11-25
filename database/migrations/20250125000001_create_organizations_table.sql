-- Migration: Create organizations table
-- Description: Multi-tenant organization accounts for billing and resource ownership
-- Phase: 3.5 Week 11 - Account Model + Organizations

CREATE TABLE organizations (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT,
    owner_id TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free' CHECK (plan IN ('free', 'starter', 'pro', 'enterprise')),
    is_personal BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_owner FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE RESTRICT
);

-- Index for owner lookups (find user's owned organizations)
CREATE INDEX idx_organizations_owner ON organizations(owner_id);

-- Index for slug lookups (unique constraint creates implicit index, but explicit for clarity)
CREATE INDEX idx_organizations_slug ON organizations(slug);

-- Partial index for active organizations by plan (for billing queries)
CREATE INDEX idx_organizations_plan ON organizations(plan) WHERE plan != 'free';

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE organizations IS 'Multi-tenant organization accounts for billing and resource ownership';
COMMENT ON COLUMN organizations.slug IS 'URL-friendly unique identifier (lowercase, hyphens allowed)';
COMMENT ON COLUMN organizations.plan IS 'Subscription plan: free, starter, pro, enterprise';
COMMENT ON COLUMN organizations.is_personal IS 'True for auto-created personal workspaces';
