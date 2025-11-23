# API Key Backfill Tool

## Overview

After implementing the scalable authentication system, existing API keys need to be regenerated to benefit from the new O(1) lookup performance. This tool helps identify which keys need attention.

## Why Can't We Auto-Backfill?

We **intentionally don't store plaintext API keys** for security reasons. We only store:
- `key_hash`: bcrypt hash (for verification)
- `key_prefix`: First 12 characters (for display only: "sk-xyz123...")

Since we can't recover the original key from the bcrypt hash, we can't generate the SHA256 lookup hash for existing keys.

## Running the Tool

```bash
cd backend
cargo run --bin backfill_key_hashes
```

## Example Output

```
🔗 Connecting to database...
✅ Connected to database

⚠️  Found 5 API keys without lookup_hash

📋 Keys that need regeneration:
   Key ID                               | Name                 | Key Prefix   | User ID             
   -----------------------------------------------------------------------------------------------
   a1b2c3d4-...                        | Production API       | sk-AbC123    | user-uuid-here
   e5f6g7h8-...                        | Development Key      | sk-XyZ789    | user-uuid-here
   i9j0k1l2-...                        | Test Integration     | sk-DeF456    | system

🔄 Action Required:
   These keys need to be regenerated to benefit from the new
   O(1) lookup performance. The old keys will continue to work
   but will use the slower fallback authentication method.

   To regenerate:
   1. Create new keys via the API: POST /auth/key/generate
   2. Update your applications with the new keys
   3. Delete the old keys via: DELETE /auth/key/{key_id}

💡 Tip: Use the API endpoints or frontend UI to manage keys.
```

## What Happens to Old Keys?

Old keys (without `key_lookup_hash`) will:
- ✅ **Still work** - they use a fallback authentication method
- ⚠️ **Be slower** - fall back to the old O(n) method for those specific keys
- 📉 **Not benefit from caching** - Redis cache only works with lookup hash

## Regeneration Process

### Via API

```bash
# 1. Generate new key
curl -X POST http://localhost:3000/auth/key/generate \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production API Key",
    "max_budget": 100.0,
    "rate_limit_rpm": 60,
    "allowed_models": ["claude-sonnet-4-5-20250929"]
  }'

# Response includes the full key (only shown once!)
{
  "id": "new-key-uuid",
  "key": "sk-NEW_KEY_HERE_SAVE_THIS",
  "key_prefix": "sk-NEW_KEY_H",
  ...
}

# 2. Update your application to use the new key

# 3. Delete the old key
curl -X DELETE http://localhost:3000/auth/key/old-key-uuid \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### Via Frontend

1. Navigate to **API Keys** page
2. Click **Generate New Key**
3. Copy the key (shown only once!)
4. Update your applications
5. Click **Delete** on the old key

## Best Practices

1. **Don't delete old keys immediately** - wait until you've verified the new ones work
2. **Update one at a time** - easier to troubleshoot if issues arise
3. **Keep a secure backup** - store new keys in your password manager
4. **Monitor after switch** - check logs for any authentication failures

## Troubleshooting

### Tool Won't Connect

```bash
# Check DATABASE_URL is set
echo $DATABASE_URL

# Or set it explicitly
DATABASE_URL=postgresql://user:pass@localhost/llm_gateway cargo run --bin backfill_key_hashes
```

### Can't Delete Old Keys

Make sure you're authenticated:
- With JWT token (for your own keys)
- With master key (for system keys)

```bash
# Get JWT token
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "you@example.com", "password": "your_password"}'
```

## Database Query (Manual Check)

```sql
-- Check how many keys need regeneration
SELECT COUNT(*) FROM virtual_keys WHERE key_lookup_hash IS NULL;

-- List keys that need regeneration
SELECT 
    id, 
    name, 
    key_prefix,
    user_id,
    created_at
FROM virtual_keys 
WHERE key_lookup_hash IS NULL
ORDER BY created_at DESC;

-- After regeneration, verify all keys have lookup_hash
SELECT COUNT(*) FROM virtual_keys WHERE key_lookup_hash IS NOT NULL;
```

## Safety Notes

⚠️ **Never run this in production without:**
1. Database backup
2. Testing in staging first
3. Communication plan with API key holders
4. Rollback plan if issues occur

✅ **Safe to run:** The tool is **read-only** - it only reports, never modifies data.
