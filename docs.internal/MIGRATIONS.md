# Database Migrations with SQLX

This project uses SQLX migrations for automatic database schema management.

## How It Works

- **Automatic**: Migrations run automatically when the application starts
- **Version-controlled**: All schema changes are tracked in `migrations/` directory
- **Idempotent**: Safe to run multiple times, SQLX tracks which migrations have been applied

## Migration Files

Migrations are stored in the `migrations/` directory:
- `20250114000000_initial_schema.sql` - Initial database schema (users, api_keys, usage tables)

## Adding a New Migration

When you need to change the database schema:

1. **Create a new migration file**:
   ```bash
   sqlx migrate add <description>
   # Example: sqlx migrate add add_user_preferences_table
   ```

2. **Edit the generated migration file** in `migrations/`:
   ```sql
   -- migrations/TIMESTAMP_add_user_preferences_table.sql
   ALTER TABLE users ADD COLUMN preferences JSONB DEFAULT '{}'::jsonb;
   CREATE INDEX idx_users_preferences ON users USING gin(preferences);
   ```

3. **Test locally** (optional):
   ```bash
   # Run against local database
   sqlx migrate run --database-url "$DATABASE_URL"

   # Revert last migration
   sqlx migrate revert --database-url "$DATABASE_URL"
   ```

4. **Commit the migration**:
   ```bash
   git add migrations/
   git commit -m "feat: add user preferences column"
   ```

5. **Deploy**:
   - Build and deploy as usual
   - Migrations run automatically when the app starts
   - No manual database changes needed

## Deployment Process

1. **Build binaries**:
   ```bash
   ./scripts/deployment/build-no-buildkit.sh
   ```

2. **Deploy with Ansible**:
   ```bash
   ansible-playbook -i ansible/inventory/hosts.yml ansible/site.yml --ask-vault-pass
   ```

3. **Migrations run automatically** when the app starts:
   ```
   [INFO] Running database migrations...
   [INFO] Database migrations completed
   [INFO] Database connection pool initialized
   ```

## Initial Setup (First Deployment)

On first deployment, SQLX will:
1. Create the `_sqlx_migrations` table to track applied migrations
2. Run all migrations in the `migrations/` directory
3. Create all tables (users, api_keys, usage) with indexes

You still need to create an initial admin user and API key manually:
```bash
# SSH to the server
ssh user@server

# Create admin user and API key
cd /home/smally/smally-api
./scripts/init_db.sh admin@example.com scale
```

**Note**: `init_db.sh` is idempotent and safe to run multiple times.

## Troubleshooting

### Migration failed during deployment
Check the application logs:
```bash
sudo journalctl -u smally -n 50
```

### Need to manually run migrations
```bash
# On the server
cd /home/smally/smally-api
export DATABASE_URL="postgresql://smally:password@localhost/smally"
sqlx migrate run
```

### Rollback a migration
SQLX doesn't support automatic rollback. Options:
1. Write a new migration that reverses the changes
2. Manually revert using SQL
3. Restore from backup

## Best Practices

1. **Always test migrations locally first** before deploying
2. **Keep migrations small and focused** - one logical change per migration
3. **Never edit existing migrations** - create a new one instead
4. **Use transactions** - SQLX wraps each migration in a transaction
5. **Plan for rollback** - consider how to reverse changes if needed

## SQLX Offline Mode

This project uses SQLX offline mode for faster builds:
- Set `SQLX_OFFLINE=true` in build environment
- Already configured in `Dockerfile`
- No database connection needed during build
