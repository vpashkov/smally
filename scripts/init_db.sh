#!/bin/bash
set -e

# Database initialization script
# Runs migrations and optionally creates an admin user

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Database connection details from environment or defaults
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5433}"
DB_USER="${DB_USER:-fastembed}"
DB_NAME="${DB_NAME:-fastembed}"
DB_PASSWORD="${DB_PASSWORD:-fastembed_dev_password}"

echo "🗄️  Initializing database..."
echo "   Host: $DB_HOST:$DB_PORT"
echo "   Database: $DB_NAME"
echo ""

# Check if database is accessible
echo "📡 Checking database connection..."
PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c '\q' 2>/dev/null || {
    echo "❌ Cannot connect to database"
    echo "   Make sure PostgreSQL is running: make services-up"
    exit 1
}
echo "✅ Database connection successful"
echo ""

# Run migrations
echo "🔄 Running migrations..."
for migration in migrations/*.sql; do
    if [ -f "$migration" ]; then
        echo "   Applying: $(basename $migration)"
        PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f "$migration" || {
            echo "❌ Migration failed: $migration"
            exit 1
        }
    fi
done
echo "✅ Migrations completed"
echo ""

# Create admin user if email provided
if [ ! -z "$1" ]; then
    EMAIL="$1"
    TIER="${2:-free}"

    echo "👤 Creating admin user..."
    echo "   Email: $EMAIL"
    echo "   Tier: $TIER"

    # Check if user already exists
    USER_EXISTS=$(PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -tAc "SELECT COUNT(*) FROM users WHERE email = '$EMAIL'")

    if [ "$USER_EXISTS" -gt 0 ]; then
        echo "⚠️  User already exists: $EMAIL"
    else
        # Generate password hash for "password123"
        PASSWORD_HASH='$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5QCZzMUpUDzqW'

        # Create user
        PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME <<SQL
INSERT INTO users (email, name, password_hash, is_active, created_at, updated_at)
VALUES ('$EMAIL', 'Admin User', '$PASSWORD_HASH', true, NOW(), NOW())
RETURNING id;
SQL

        echo "✅ Admin user created"
        echo "   Password: password123"
        echo "   ⚠️  Please change this password after first login!"
    fi
fi

echo ""
echo "🎉 Database initialization complete!"
echo ""
echo "Next steps:"
echo "  1. Start the server: cargo run"
echo "  2. Open browser: http://localhost:8000"
echo "  3. Register a new account or login with admin credentials"
