# Deployment Migration Guide: Go → Rust

This document explains the changes needed to deploy the Rust version of the Smally API.

## What Changed

The Smally API was rewritten from **Go** to **Rust** for better performance and lower latency.

## Required Changes

### 1. Update Ansible Inventory Variable

In `ansible/group_vars/all/vault.yml` or your inventory file, update the git repository URL:

**Before (Go version):**
```yaml
smally_git_repo: "git@github.com:yourusername/smally-go.git"
```

**After (Rust version):**
```yaml
smally_git_repo: "git@github.com:yourusername/embed.git"  # Or your Rust repo URL
smally_git_branch: "main"  # Make sure this points to the Rust version
```

### 2. Files Already Updated

The following files have been updated to work with the Rust version:

- ✅ **Dockerfile** - Multi-stage build for Rust binary
- ✅ **scripts/deployment/deploy.sh** - Uses Rust init_db.sh instead of Python
- ✅ **scripts/init_db.sh** - Bash script that creates tables and users
- ✅ **Makefile** - Rust cargo commands

### 3. Deployment Architecture

**Container Structure:**
```
app container:
  /app/api              - Main API server (Rust binary)
  /app/create_api_key        - CLI tool to create API keys (Rust binary)
  /app/scripts/init_db.sh    - Database initialization (Bash script)
```

**Deployment Flow:**
1. Ansible clones Rust repo
2. Deployment script downloads ONNX model files from Hugging Face (~86MB)
3. Dockerfile builds Rust binaries (multi-stage build)
4. Docker Compose starts services
5. Model files are copied into the container volume
6. `init_db.sh` creates tables and admin user
7. Systemd manages the service

### 4. Database Initialization

The database initialization now uses:

**Local development:**
```bash
make init-db
# Runs: ./scripts/init_db.sh admin@example.com scale
```

**Production (via Ansible):**
```bash
docker-compose -f docker-compose.prod.yml exec -T app /app/scripts/init_db.sh admin@example.com scale
```

**Manual API key creation:**
```bash
# Inside the container
/app/create_api_key <email> <tier> <name>

# Or via docker-compose
docker-compose -f docker-compose.prod.yml exec app /app/create_api_key user@example.com pro "My API Key"
```

### 5. Environment Variables

The `.env` file format remains the same. The Rust version uses the same environment variables as the Go version:

```bash
# Database
DATABASE_URL=postgres://user:pass@postgres:5432/smally

# Redis
REDIS_URL=redis://redis:6379

# API Configuration
API_KEY_PREFIX=fe_
FREE_TIER_LIMIT=10000
PRO_TIER_LIMIT=1000000
SCALE_TIER_LIMIT=10000000

# Model
MODEL_NAME=sentence-transformers/all-MiniLM-L6-v2
MODEL_PATH=./models/all-MiniLM-L6-v2-onnx
MAX_TOKENS=512

# Cache
L1_CACHE_SIZE=1000
L2_CACHE_TTL=3600
```

### 6. Health Check

The health endpoint now includes build information:

```bash
curl http://localhost:8000/health
```

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "model": "sentence-transformers/all-MiniLM-L6-v2",
  "build": {
    "git_hash": "abc1234",
    "git_branch": "main",
    "git_date": "2025-11-13 13:28:33 +0400",
    "git_dirty": false,
    "build_timestamp": "2025-11-13T14:36:28.928671+00:00",
    "rust_version": "1.91.0",
    "profile": "release"
  }
}
```

### 7. Performance Improvements

The Rust version includes several optimizations:

- **API key caching** - In-memory cache with 5-minute TTL
- **Redis connection pooling** - Persistent connections for rate limiting
- **Batch usage tracking** - 5-second flush interval
- **Faster hashing** - seahash for cache keys
- **No monthly quotas for paid tiers** - Pure pay-as-you-go

**Expected latency:**
- Free tier: p95 ~6ms (with rate limiting)
- Paid tier: p95 ~1-2ms (no rate limiting)

### 8. Deployment Command

Deploy with Ansible:

```bash
ansible-playbook -i ansible/inventory/hosts.yml ansible/playbook.yml --ask-vault-pass
```

Quick deploy (code changes only):

```bash
ansible-playbook -i ansible/inventory/hosts.yml ansible/quick-deploy.yml --ask-vault-pass
```

### 9. Model Files

The deployment script automatically downloads the ONNX model files from Hugging Face:
- Model: sentence-transformers/all-MiniLM-L6-v2
- Size: ~86MB total
- Location: `models/all-MiniLM-L6-v2-onnx/`

Files downloaded:
- `model.onnx` - The ONNX runtime model
- `vocab.txt` - Vocabulary file
- `tokenizer.json` - Tokenizer configuration
- `tokenizer_config.json` - Tokenizer settings
- `config.json` - Model configuration
- `special_tokens_map.json` - Special tokens mapping

The model files are:
1. Downloaded to the host's `models/` directory (if not already present)
2. Copied into the Docker volume `app_models:/app/models`
3. Used by the application at runtime

**Manual download (if needed):**
```bash
make model
```

### 10. Troubleshooting

**Issue:** "Failed to connect to database" during init_db

**Solution:** The error message shows `lookup postgres on 127.0.0.11:53` which means:
- The script is trying to resolve hostname "postgres"
- Inside the container, it should resolve to the postgres service
- Make sure you're running init_db from inside the app container:
  ```bash
  docker-compose -f docker-compose.prod.yml exec -T app /app/scripts/init_db.sh admin@example.com scale
  ```

**Issue:** Cannot find create_api_key binary

**Solution:** Rebuild the Docker image to include the binary:
```bash
docker-compose -f docker-compose.prod.yml build --no-cache app
```

**Issue:** "Model file not found" or inference errors

**Solution:** The model files may not have been downloaded or copied correctly:
```bash
# Check if model files exist on host
ls -lh models/all-MiniLM-L6-v2-onnx/

# Check if model files exist in container
docker-compose -f docker-compose.prod.yml exec app ls -lh /app/models/all-MiniLM-L6-v2-onnx/

# If missing, manually copy them
docker cp models/all-MiniLM-L6-v2-onnx smally-api:/app/models/
```

**Issue:** Deployment script fails with "ignore_errors"

**Solution:** The Ansible playbook previously had `ignore_errors: yes` which masked deployment failures. This has been removed. Check the actual error in the deployment logs and fix the root cause.

### 11. Rollback Plan

If you need to rollback to the Go version:

1. Update `smally_git_repo` back to the Go repository
2. Run Ansible playbook again
3. The Go version will be deployed

### 12. Verification

After deployment, verify:

```bash
# Check health
curl http://localhost:8000/health

# Check build info shows Rust
curl http://localhost:8000/health | jq '.build'

# Test embedding
curl -X POST http://localhost:8000/v1/embed \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello world", "normalize": true}'
```

## Summary

The migration from Go to Rust is straightforward:
1. Update the git repository URL in Ansible inventory
2. Deploy with Ansible (it will automatically use the new Dockerfile and scripts)
3. Verify the deployment with health checks

All deployment scripts and Docker configuration have been updated to support the Rust version seamlessly.
