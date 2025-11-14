-- Migration: Add organizations and multi-tenancy support
-- This migration transforms the schema from user-based to organization-based API keys

-- Step 1: Add new columns to users table
ALTER TABLE users
ADD COLUMN IF NOT EXISTS name VARCHAR(255),
ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255);

-- Step 2: Create organizations table
CREATE TABLE IF NOT EXISTS organizations (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    owner_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier VARCHAR(50) NOT NULL DEFAULT 'free',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Step 3: Create organization_members table (many-to-many relationship)
CREATE TABLE IF NOT EXISTS organization_members (
    id BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member', -- owner, admin, member
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, user_id)
);

-- Step 4: Create temporary migration table to track user -> organization mapping
CREATE TABLE IF NOT EXISTS temp_user_org_mapping (
    user_id BIGINT PRIMARY KEY,
    organization_id BIGINT NOT NULL
);

-- Step 5: Create personal organizations for existing users
-- For each existing user, create a personal organization
INSERT INTO organizations (name, slug, owner_id, tier, is_active, created_at, updated_at)
SELECT
    COALESCE(email, 'User ' || id) || '''s Organization',
    'user-' || id || '-org',
    id,
    tier,
    is_active,
    created_at,
    updated_at
FROM users
WHERE NOT EXISTS (
    SELECT 1 FROM organizations WHERE owner_id = users.id
);

-- Step 6: Map users to their personal organizations
INSERT INTO temp_user_org_mapping (user_id, organization_id)
SELECT u.id, o.id
FROM users u
JOIN organizations o ON o.owner_id = u.id AND o.slug = 'user-' || u.id || '-org';

-- Step 7: Add organization members for each user (owner role)
INSERT INTO organization_members (organization_id, user_id, role, created_at)
SELECT organization_id, user_id, 'owner', NOW()
FROM temp_user_org_mapping
ON CONFLICT (organization_id, user_id) DO NOTHING;

-- Step 8: Add organization_id to api_keys table
ALTER TABLE api_keys
ADD COLUMN IF NOT EXISTS organization_id BIGINT,
ADD COLUMN IF NOT EXISTS key_id UUID;

-- Step 9: Migrate existing api_keys to use organization_id
UPDATE api_keys ak
SET organization_id = m.organization_id
FROM temp_user_org_mapping m
WHERE ak.user_id = m.user_id
AND ak.organization_id IS NULL;

-- Step 10: Generate UUIDs for existing api_keys that don't have them
UPDATE api_keys
SET key_id = gen_random_uuid()
WHERE key_id IS NULL;

-- Step 11: Make organization_id and key_id NOT NULL after migration
ALTER TABLE api_keys
ALTER COLUMN organization_id SET NOT NULL,
ALTER COLUMN key_id SET NOT NULL;

-- Step 12: Add foreign key constraint
ALTER TABLE api_keys
ADD CONSTRAINT fk_api_keys_organization_id
FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;

-- Step 13: Update usage table to use organization_id
ALTER TABLE usage
ADD COLUMN IF NOT EXISTS organization_id BIGINT;

-- Step 14: Migrate existing usage records
UPDATE usage u
SET organization_id = m.organization_id
FROM temp_user_org_mapping m
WHERE u.user_id = m.user_id
AND u.organization_id IS NULL;

-- Step 15: Make organization_id NOT NULL
ALTER TABLE usage
ALTER COLUMN organization_id SET NOT NULL;

-- Step 16: Add foreign key constraint
ALTER TABLE usage
ADD CONSTRAINT fk_usage_organization_id
FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;

-- Step 17: Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_organizations_owner_id ON organizations(owner_id);
CREATE INDEX IF NOT EXISTS idx_organizations_slug ON organizations(slug);
CREATE INDEX IF NOT EXISTS idx_organization_members_org_id ON organization_members(organization_id);
CREATE INDEX IF NOT EXISTS idx_organization_members_user_id ON organization_members(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_organization_id ON api_keys(organization_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_id ON api_keys(key_id);
CREATE INDEX IF NOT EXISTS idx_usage_organization_id ON usage(organization_id);

-- Step 18: Drop temporary mapping table
DROP TABLE IF EXISTS temp_user_org_mapping;

-- Step 19: Remove tier from users table (now in organizations)
-- We keep it for backward compatibility but it's no longer the source of truth
-- ALTER TABLE users DROP COLUMN tier; -- Commented out for safety

-- Migration complete!
-- Summary:
-- - Users can now belong to multiple organizations
-- - Each organization has its own tier and API keys
-- - Existing users have been migrated to personal organizations
-- - API keys are now scoped to organizations instead of users
