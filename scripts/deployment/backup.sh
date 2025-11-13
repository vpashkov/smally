#!/bin/bash
set -euo pipefail

# Smally Database Backup Script
# Creates a backup of PostgreSQL database

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BACKUP_DIR="$PROJECT_ROOT/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="smally_backup_${TIMESTAMP}.sql.gz"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cd "$PROJECT_ROOT"

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

log_info "Starting database backup..."

# Source environment variables
source .env

# Create backup
docker-compose -f docker-compose.prod.yml exec -T postgres \
    pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" | gzip > "$BACKUP_DIR/$BACKUP_FILE"

if [ $? -eq 0 ]; then
    log_info "Backup created successfully: $BACKUP_FILE"
    log_info "Size: $(du -h "$BACKUP_DIR/$BACKUP_FILE" | cut -f1)"

    # Keep only last 30 backups
    cd "$BACKUP_DIR"
    ls -t smally_backup_*.sql.gz | tail -n +31 | xargs -r rm
    log_info "Old backups cleaned up (keeping last 30)"
else
    log_error "Backup failed"
    exit 1
fi
