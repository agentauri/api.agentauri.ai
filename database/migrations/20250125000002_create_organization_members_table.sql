-- Migration: Create organization_members table
-- Description: Organization membership with role-based access control
-- Phase: 3.5 Week 11 - Account Model + Organizations

CREATE TABLE organization_members (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    organization_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
    invited_by TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_organization FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_invited_by FOREIGN KEY (invited_by) REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT unique_org_user UNIQUE (organization_id, user_id)
);

-- Index for finding all organizations a user belongs to
CREATE INDEX idx_org_members_user ON organization_members(user_id);

-- Index for finding all members of an organization
CREATE INDEX idx_org_members_org ON organization_members(organization_id);

-- Composite index for role-based queries within an organization
CREATE INDEX idx_org_members_org_role ON organization_members(organization_id, role);

COMMENT ON TABLE organization_members IS 'Organization membership with role-based access control';
COMMENT ON COLUMN organization_members.role IS 'Role hierarchy: owner > admin > member > viewer';
COMMENT ON COLUMN organization_members.invited_by IS 'User who invited this member (NULL for owner)';
