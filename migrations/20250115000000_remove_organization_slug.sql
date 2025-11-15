-- Remove slug column from organizations table
-- Slugs are not used in URLs (we use UUIDs instead)

-- Drop the unique index first
DROP INDEX IF EXISTS idx_organizations_slug;

-- Drop the slug column
ALTER TABLE organizations DROP COLUMN slug;
