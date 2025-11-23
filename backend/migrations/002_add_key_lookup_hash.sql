-- Migration: Add key_lookup_hash column for fast key authentication
-- This enables O(1) database lookups instead of O(n) bcrypt comparisons

-- Add the key_lookup_hash column if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'virtual_keys' AND column_name = 'key_lookup_hash'
    ) THEN
        ALTER TABLE virtual_keys ADD COLUMN key_lookup_hash VARCHAR(64);
    END IF;
END $$;

-- Create index for fast lookups
CREATE INDEX IF NOT EXISTS idx_virtual_keys_key_lookup_hash
ON virtual_keys(key_lookup_hash)
WHERE key_lookup_hash IS NOT NULL;

-- Note: Existing keys cannot be backfilled automatically because we don't store
-- the plain-text keys (only bcrypt hashes). New keys will have lookup_hash populated.
--
-- Recommendation: Users should regenerate their API keys after this migration
-- to benefit from the performance improvements.

COMMENT ON COLUMN virtual_keys.key_lookup_hash IS 'SHA256 hash of the key for fast O(1) lookups (not for security, bcrypt key_hash is still used for verification)';
