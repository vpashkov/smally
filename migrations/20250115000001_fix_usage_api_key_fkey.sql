-- Fix usage table foreign key to reference api_keys.key_id instead of api_keys.id
-- This allows us to directly use the key_id from the token without looking up the primary key

-- Drop the existing foreign key constraint
ALTER TABLE usage DROP CONSTRAINT IF EXISTS usage_api_key_id_fkey;

-- Add new foreign key constraint referencing key_id
ALTER TABLE usage ADD CONSTRAINT usage_api_key_id_fkey
    FOREIGN KEY (api_key_id) REFERENCES api_keys(key_id) ON DELETE SET NULL;
