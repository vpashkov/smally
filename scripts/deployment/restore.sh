#!/bin/bash
set -euo pipefail

# FastEmbed Database Restore Script
# Restores PostgreSQL database from backup

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BACKUP_DIR="$PROJECT_ROOT/backups"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check arguments
if [ $# -eq 0 ]; then
    log_error "Usage: $0 <backup_file>"
    log_info "Available backups:"
    ls -lh "$BACKUP_DIR"/fastembed_backup_*.sql.gz 2>/dev/null || echo "  No backups found"
    exit 1
fi

BACKUP_FILE="$1"

if [[ ! "$BACKUP_FILE" = /* ]]; then
    BACKUP_FILE="$BACKUP_DIR/$BACKUP_FILE"
fi

if [[ ! -f "$BACKUP_FILE" ]]; then
    log_error "Backup file not found: $BACKUP_FILE"
    exit 1
fi

cd "$PROJECT_ROOT"

# Source environment variables
source .env

log_warn "WARNING: This will overwrite the current database!"
read -p "Are you sure you want to continue? (yes/no): " confirm

if [[ "$confirm" != "yes" ]]; then
    log_info "Restore cancelled"
    exit 0
fi

log_info "Stopping application..."
docker-compose -f docker-compose.prod.yml stop app

log_info "Restoring database from: $BACKUP_FILE"

# Drop and recreate database
docker-compose -f docker-compose.prod.yml exec -T postgres psql -U "$POSTGRES_USER" -d postgres <<EOF
DROP DATABASE IF EXISTS $POSTGRES_DB;
CREATE DATABASE $POSTGRES_DB;
EOF

# Restore backup
gunzip -c "$BACKUP_FILE" | docker-compose -f docker-compose.prod.yml exec -T postgres \
    psql -U "$POSTGRES_USER" "$POSTGRES_DB"

if [ $? -eq 0 ]; then
    log_info "Database restored successfully"
    log_info "Restarting application..."
    docker-compose -f docker-compose.prod.yml start app
    log_info "Restore complete"
else
    log_error "Restore failed"
    exit 1
fi
