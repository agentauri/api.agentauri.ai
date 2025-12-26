-- Make organization slug user-scoped instead of globally unique
--
-- This allows different users to have organizations with the same slug.
-- For example, user A can have org "my-project" and user B can also have "my-project".
-- The slug only needs to be unique within the same owner's organizations.

-- Drop the existing global unique constraint on slug
ALTER TABLE organizations DROP CONSTRAINT IF EXISTS organizations_slug_key;

-- Drop the existing index on slug (will be replaced by composite index)
DROP INDEX IF EXISTS idx_organizations_slug;

-- Add new composite unique constraint: slug must be unique per owner
ALTER TABLE organizations ADD CONSTRAINT organizations_owner_slug_key UNIQUE (owner_id, slug);

-- Add composite index for efficient lookups by owner + slug
CREATE INDEX IF NOT EXISTS idx_organizations_owner_slug ON organizations (owner_id, slug);
