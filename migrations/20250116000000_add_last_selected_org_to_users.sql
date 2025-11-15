-- Add last_selected_org_id to users table
-- This allows us to redirect users to their last visited organization after login

ALTER TABLE users
ADD COLUMN last_selected_org_id UUID REFERENCES organizations(id) ON DELETE SET NULL;

-- Create index for performance
CREATE INDEX idx_users_last_selected_org_id ON users(last_selected_org_id);

-- Update existing users to set their last_selected_org_id to their owned organization (if any)
UPDATE users
SET last_selected_org_id = (
    SELECT o.id
    FROM organizations o
    WHERE o.owner_id = users.id
    LIMIT 1
)
WHERE last_selected_org_id IS NULL;
