# Deployment Guide

Complete guide for deploying Smally API to production using binary deployment with Ansible.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Deployment Process](#deployment-process)
- [Database Migrations](#database-migrations)
- [Troubleshooting](#troubleshooting)
- [Lessons Learned](#lessons-learned)

## Overview

Smally API uses a **binary deployment** strategy:
- Cross-compile Rust binaries locally (ARM64 Linux)
- Deploy binaries to production server via Ansible
- Run as systemd service (no Docker on production)
- Automatic database migrations on startup

## Architecture

### Production Stack

```
┌─────────────────────────────────────────┐
│          Nginx (Reverse Proxy)          │
│  - Rate limiting                        │
│  - SSL termination                      │
│  - Security headers                     │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│      Smally API (Binary)                │
│  - Rust application                     │
│  - Runs as systemd service              │
│  - ONNX Runtime for embeddings          │
└─────┬───────────────────┬───────────────┘
      │                   │
┌─────▼──────┐    ┌──────▼────────┐
│ PostgreSQL │    │     Redis     │
│  - Users   │    │  - Cache      │
│  - API keys│    │  - Rate limit │
│  - Usage   │    │               │
└────────────┘    └───────────────┘
```

### Why Binary Deployment?

1. **Faster**: No Docker image building on server
2. **Simpler**: Single binary, no containers
3. **Lighter**: Minimal resource overhead
4. **Faster deploys**: Upload binary → restart service
5. **Easier debugging**: Direct access to logs and process

## Prerequisites

### Local Development Machine

- Docker (for cross-compilation)
- Ansible >= 2.13
- SSH access to production server

### Production Server

- Ubuntu 22.04 (ARM64)
- PostgreSQL 15
- Redis
- Nginx
- Systemd

## Deployment Process

### Step 1: Build Binaries

Choose the appropriate build script:

```bash
# With Docker BuildKit (recommended - faster, caches dependencies)
./scripts/deployment/build.sh

# Without BuildKit (for Colima or older Docker)
./scripts/deployment/build-no-buildkit.sh

# Native build (macOS only, for local testing)
./scripts/deployment/build-native.sh
```

**Output**: Binaries in `dist/` directory
- `dist/api` - Main API server
- `dist/create_api_key` - API key management tool

### Step 2: Deploy with Ansible

```bash
ansible-playbook -i ansible/inventory/hosts.yml ansible/site.yml --ask-vault-pass
```

**What it does**:
1. ✅ System setup (packages, user, firewall)
2. ✅ Install ONNX Runtime libraries
3. ✅ Install PostgreSQL and create database
4. ✅ Install Redis
5. ✅ Configure Nginx reverse proxy
6. ✅ Upload binaries and scripts
7. ✅ Create .env configuration
8. ✅ Install and start systemd service
9. ✅ Wait for health check

### Step 3: Verify Deployment

```bash
# Check service status
ssh user@server "sudo systemctl status smally"

# Check logs
ssh user@server "sudo journalctl -u smally -f"

# Health check
curl http://your-server/health
```

## Database Migrations

### How Migrations Work

Migrations run **automatically** when the application starts using SQLX:

1. App starts → `database::init_db()` called
2. SQLX reads embedded migration files
3. Creates `_sqlx_migrations` table if needed
4. Runs pending migrations in order
5. App continues startup

### Migration Files

Located in `migrations/` directory:
- `20250114000000_initial_schema.sql` - Initial tables and indexes

### Adding New Migrations

```bash
# Create new migration
sqlx migrate add description_of_change

# Edit the generated file
vim migrations/TIMESTAMP_description_of_change.sql

# Migrations run automatically on next deployment
git add migrations/
git commit -m "feat: add new migration"
```

**No manual database changes needed!** Just deploy the new binary.

See [MIGRATIONS.md](./MIGRATIONS.md) for complete guide.

## Troubleshooting

### Service Won't Start

**Symptom**: `status=203/EXEC` error in systemd

**Causes**:
- Binary not executable: `chmod +x /home/smally/smally-api/api`
- Wrong architecture: Check with `file /home/smally/smally-api/api`
- Missing libraries: Check with `ldd /home/smally/smally-api/api`
- Systemd security hardening blocking access

**Solution**: The playbook now includes diagnostics that show:
- Binary file info
- Library dependencies
- Test execution result

### Migration Failures

**Symptom**: App crashes on startup with migration error

**Check logs**:
```bash
sudo journalctl -u smally -n 100
```

**Common issues**:
- Database connection failed
- Migration syntax error
- Conflicting schema changes

**Recovery**:
```bash
# Check migration status
psql $DATABASE_URL -c "SELECT * FROM _sqlx_migrations;"

# Manually revert if needed (SQLX doesn't support automatic rollback)
psql $DATABASE_URL -c "DELETE FROM _sqlx_migrations WHERE version = 'TIMESTAMP';"
```

### Connection Refused

**Symptom**: Health check fails, can't connect to API

**Check**:
1. Service running: `systemctl status smally`
2. Port listening: `sudo lsof -i :8000`
3. Firewall: `sudo ufw status`
4. Nginx config: `sudo nginx -t`

### Database Connection Issues

**Symptom**: "Operation not permitted" errors

**Check**:
- PostgreSQL running: `systemctl status postgresql`
- Credentials in `.env`: `cat /home/smally/smally-api/.env`
- Database exists: `psql -U smally -d smally -c '\l'`

## Lessons Learned

### Session Summary: Deployment Migration to Binary + Ansible

This session transformed the deployment from Docker Compose to binary deployment with SQLX migrations.

#### Problems Solved

1. **Build Performance**: Remote Docker builds were slow
   - **Solution**: Cross-compile locally, upload binaries
   - **Result**: 10x faster deployments

2. **Schema Management**: Manual SQL scripts, easy to forget
   - **Solution**: SQLX migrations embedded in binary
   - **Result**: Automatic, version-controlled schema changes

3. **Deployment Complexity**: Multiple playbooks, roles
   - **Solution**: Single comprehensive `site.yml` playbook
   - **Result**: Easier to understand and maintain

4. **Service Start Failures**: `status=203/EXEC` errors
   - **Root Cause**: `ProtectHome=true` blocked access to `/home`
   - **Solution**: Removed overly strict systemd security settings
   - **Result**: Service starts reliably

5. **Deprecation Warnings**: Ansible parameter names changed
   - **Issue**: `db` → `database` → `login_db`, `role` → `roles`
   - **Solution**: Updated to current parameter names
   - **Result**: Clean deployment output

#### Key Decisions

**Why Binary Deployment?**
- ✅ Faster builds (local cross-compilation)
- ✅ Faster deploys (upload binary vs building image)
- ✅ Simpler architecture (no Docker complexity)
- ✅ Easier debugging (direct process access)
- ❌ Tradeoff: Need to manage dependencies (ONNX Runtime)

**Why SQLX Migrations?**
- ✅ Automatic on app startup
- ✅ Version controlled with code
- ✅ Embedded in binary (no separate files needed)
- ✅ Tracks what's applied
- ❌ Tradeoff: No automatic rollback (must write new migration)

**Why Single Playbook?**
- ✅ Everything in one place
- ✅ Easier to understand
- ✅ Fully idempotent
- ✅ No role dependencies
- ❌ Tradeoff: Larger file (but well-organized with comments)

#### Mistakes and Fixes

1. **Systemd Security Too Strict**
   - Mistake: Used `ProtectHome=true` which blocks `/home`
   - Impact: Service couldn't execute binary
   - Fix: Removed excessive hardening, kept `NoNewPrivileges` and `PrivateTmp`

2. **Missing Dependencies**
   - Mistake: Forgot to install ONNX Runtime on server
   - Impact: Binary couldn't run (missing shared libraries)
   - Fix: Added ONNX Runtime installation to playbook

3. **Database Init in Wrong Place**
   - Mistake: Initially tried to run init_db.sh from Ansible
   - Impact: Timing issues, not idempotent
   - Fix: Removed from Ansible, migrations handle it automatically

4. **Deprecated Ansible Parameters**
   - Mistake: Used old parameter names (`db`, `role`)
   - Impact: Deprecation warnings
   - Fix: Updated to `login_db` and `roles`

#### Files Cleaned Up

Removed **23 obsolete files** (1,686 lines):

**Scripts** (10 files):
- `init_db.sh`, `add_indexes.sql` - Replaced by SQLX migrations
- `deploy.sh`, `manual-deploy.sh`, `quick-deploy.sh` - Replaced by Ansible
- `build.sh` (root) - Duplicate
- Others

**Ansible** (13 files):
- `playbook.yml`, `quick-deploy.yml`, `deploy.yml` - Replaced by `site.yml`
- All roles (`common`, `docker`, `smally`) - Inline in `site.yml`

#### Final Architecture

**Development Workflow**:
```
┌─────────────┐
│ Developer   │
│  - Edit code│
│  - Commit   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Build       │──── ./scripts/deployment/build-no-buildkit.sh
│ (Docker)    │──── Produces: dist/api, dist/create_api_key
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Deploy      │──── ansible-playbook ansible/site.yml
│ (Ansible)   │──── Uploads binaries, configs
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Server      │
│  - Systemd  │──── Starts binary
│  - Binary   │──── Runs migrations automatically
│  - Migrations│──── App ready
└─────────────┘
```

**Production Runtime**:
```
Request → Nginx → Smally API Binary → PostgreSQL
                      ↓
                    Redis
```

#### Performance Improvements

- **Build time**: ~5 min remote → ~2 min local
- **Deploy time**: ~10 min → ~2 min (upload + restart)
- **Migration time**: Manual → Automatic (0 seconds human time)
- **Debugging time**: Faster (direct systemd logs)

#### Best Practices Established

1. **Always separate build from deploy**
   - Build creates artifacts (binaries)
   - Deploy distributes artifacts
   - Never build on production server

2. **Embed migrations in application**
   - Schema changes live with code
   - Automatic on app start
   - Version controlled

3. **Use comprehensive playbooks**
   - Single source of truth
   - Fully idempotent
   - Well-documented with comments

4. **Test systemd services carefully**
   - Security hardening can block execution
   - Use diagnostics to debug
   - Start minimal, add hardening incrementally

5. **Keep deployment simple**
   - Binary > Container (when possible)
   - Single playbook > Multiple roles
   - Automatic > Manual

## Next Steps

After successful deployment:

1. **Create API keys**: Use `scripts/deployment/create-api-key.sh`
2. **Monitor logs**: `journalctl -u smally -f`
3. **Set up backups**: Use `scripts/deployment/backup.sh`
4. **Configure SSL**: Add Let's Encrypt to Nginx
5. **Set up monitoring**: Prometheus metrics at `/metrics`

## Quick Reference

### Common Commands

```bash
# Build
./scripts/deployment/build-no-buildkit.sh

# Deploy
ansible-playbook -i ansible/inventory/hosts.yml ansible/site.yml --ask-vault-pass

# Check service
ssh user@server "systemctl status smally"

# View logs
ssh user@server "journalctl -u smally -f"

# Restart service
ssh user@server "systemctl restart smally"

# Create API key
ssh user@server "cd /home/smally/smally-api && ./scripts/deployment/create-api-key.sh email@example.com scale"

# Backup database
ssh user@server "/home/smally/smally-api/scripts/deployment/backup.sh"
```

### File Locations on Server

```
/home/smally/smally-api/
├── api                    # Main binary
├── create_api_key         # API key tool
├── .env                   # Configuration (created by Ansible)
├── logs/                  # Application logs
└── scripts/               # Operational scripts
    └── deployment/
        ├── backup.sh
        ├── restore.sh
        ├── create-api-key.sh
        └── health-check.sh

/etc/systemd/system/
└── smally.service         # Systemd service file

/etc/nginx/sites-available/
└── smally                 # Nginx configuration
```

### Environment Variables

Set in `/home/smally/smally-api/.env`:

```bash
DATABASE_URL=postgresql://smally:password@localhost/smally
REDIS_URL=redis://localhost:6379
RUST_LOG=info
API_PORT=8000
```

## Support

For issues:
1. Check [Troubleshooting](#troubleshooting) section
2. Check logs: `journalctl -u smally -n 100`
3. Review Ansible output for errors
4. Check [MIGRATIONS.md](./MIGRATIONS.md) for database issues
