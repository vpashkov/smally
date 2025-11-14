-- Migration to remove/nullify legacy API key fields
-- The CWT token-based approach doesn't use key_hash or key_prefix
-- user_id is kept for backward compatibility but made nullable

-- Make legacy fields nullable
ALTER TABLE api_keys
ALTER COLUMN user_id DROP NOT NULL,
ALTER COLUMN key_hash DROP NOT NULL,
ALTER COLUMN key_prefix DROP NOT NULL;

-- Add default empty string for key_hash and key_prefix for backward compatibility
ALTER TABLE api_keys
ALTER COLUMN key_hash SET DEFAULT '',
ALTER COLUMN key_prefix SET DEFAULT '';
