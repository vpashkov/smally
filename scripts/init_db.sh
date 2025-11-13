#!/bin/bash
set -euo pipefail

# Smally Database Initialization Script
# Creates database tables and admin user with API key
#
# Usage:
#   Local: ./scripts/init_db.sh admin@example.com scale
#   Docker: docker-compose -f docker-compose.prod.yml exec -T postgres /backups/init_db.sh admin@example.com scale

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get arguments
EMAIL="${1:-admin@example.com}"
TIER="${2:-scale}"

echo -e "${YELLOW}Smally Database Initialization${NC}"
echo "=================================="
echo ""

# Load environment variables from .env if it exists
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Parse DATABASE_URL if set, otherwise use individual variables
if [ -n "${DATABASE_URL:-}" ]; then
    # Extract components from DATABASE_URL
    # Format: postgres://user:password@host:port/database
    DB_USER=$(echo "$DATABASE_URL" | sed -n 's/.*:\/\/\([^:]*\):.*/\1/p')
    DB_PASS=$(echo "$DATABASE_URL" | sed -n 's/.*:\/\/[^:]*:\([^@]*\)@.*/\1/p')
    DB_HOST=$(echo "$DATABASE_URL" | sed -n 's/.*@\([^:]*\):.*/\1/p')
    DB_PORT=$(echo "$DATABASE_URL" | sed -n 's/.*:\([0-9]*\)\/.*/\1/p')
    DB_NAME=$(echo "$DATABASE_URL" | sed -n 's/.*\/\([^?]*\).*/\1/p')
else
    DB_USER="${POSTGRES_USER:-smally}"
    DB_PASS="${POSTGRES_PASSWORD:-}"
    DB_HOST="${DB_HOST:-localhost}"
    DB_PORT="${DB_PORT:-5432}"
    DB_NAME="${POSTGRES_DB:-smally}"
fi

# Check if we can connect
echo -e "${YELLOW}→${NC} Checking database connection..."
if PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c '\q' 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Connected to database"
else
    echo -e "${RED}✗${NC} Failed to connect to database"
    exit 1
fi

# Create tables
echo -e "${YELLOW}→${NC} Creating database tables..."
PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    tier VARCHAR(50) NOT NULL DEFAULT 'free',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- API Keys table
CREATE TABLE IF NOT EXISTS api_keys (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    name VARCHAR(255) NOT NULL DEFAULT 'Default API Key',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMP,
    UNIQUE(key_hash)
);

-- Usage tracking table
CREATE TABLE IF NOT EXISTS usage (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id BIGINT REFERENCES api_keys(id) ON DELETE SET NULL,
    embeddings_count INTEGER NOT NULL DEFAULT 0,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_usage_user_id ON usage(user_id);
CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage(timestamp);
EOF

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Tables created successfully"
else
    echo -e "${RED}✗${NC} Failed to create tables"
    exit 1
fi

# Generate API key (32 bytes = 64 hex chars)
API_KEY="fe_$(openssl rand -hex 32)"
KEY_HASH=$(echo -n "$API_KEY" | sha256sum | cut -d' ' -f1)
KEY_PREFIX="${API_KEY:0:13}..."

# Create admin user and API key
echo -e "${YELLOW}→${NC} Creating admin user..."
PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" << EOF
-- Insert or get user
INSERT INTO users (email, tier, is_active, created_at, updated_at)
VALUES ('$EMAIL', '$TIER', true, NOW(), NOW())
ON CONFLICT (email) DO UPDATE SET tier = '$TIER', updated_at = NOW()
RETURNING id;

-- Get user ID
\set user_id (SELECT id FROM users WHERE email = '$EMAIL')

-- Insert API key
INSERT INTO api_keys (user_id, key_hash, key_prefix, name, is_active, created_at)
VALUES (
    (SELECT id FROM users WHERE email = '$EMAIL'),
    '$KEY_HASH',
    '$KEY_PREFIX',
    'Default API Key',
    true,
    NOW()
);
EOF

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Admin user created: $EMAIL (Tier: $TIER)"
else
    echo -e "${RED}✗${NC} Failed to create admin user"
    exit 1
fi

# Display API key
echo ""
echo "============================================================"
echo "API KEY GENERATED (save this, it won't be shown again):"
echo "============================================================"
echo ""
echo "$API_KEY"
echo ""
echo "============================================================"
echo ""
echo "Add this to your requests as:"
echo "Authorization: Bearer $API_KEY"
echo "============================================================"
echo ""
echo -e "${GREEN}Database initialization complete!${NC}"
