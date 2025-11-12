#!/bin/bash
set -euo pipefail

# FastEmbed API - Generate Production Environment Variables
# This script generates secure passwords for .env

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
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

cd "$PROJECT_ROOT"

# Use .env.production.example as template, but output to .env
ENV_TEMPLATE=".env.production.example"
ENV_FILE=".env"

# Check if template exists
if [[ ! -f "$ENV_TEMPLATE" ]]; then
  log_error "$ENV_TEMPLATE not found"
  exit 1
fi

# Generate secure passwords
POSTGRES_PASSWORD=$(openssl rand -base64 32)
GRAFANA_PASSWORD=$(openssl rand -base64 24)
SECRET_KEY=$(openssl rand -hex 32)

echo ""
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}FastEmbed API - Production Configuration${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""

# Check if .env already exists
if [[ -f "$ENV_FILE" ]]; then
  log_warn "$ENV_FILE already exists!"
  echo ""
  read -p "Do you want to update it with new passwords? (y/N): " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_info "Cancelled. No changes made."
    exit 0
  fi

  # Backup existing file
  BACKUP_FILE=".env.backup.$(date +%Y%m%d_%H%M%S)"
  cp "$ENV_FILE" "$BACKUP_FILE"
  log_info "Backed up existing file to: $BACKUP_FILE"
fi

# Copy template
cp "$ENV_TEMPLATE" "$ENV_FILE"

# Replace placeholder passwords with generated ones
sed -i.tmp "s/CHANGE_TO_SECURE_PASSWORD/$POSTGRES_PASSWORD/g" "$ENV_FILE"
sed -i.tmp "s/GENERATE_SECURE_RANDOM_KEY/$SECRET_KEY/g" "$ENV_FILE"

# Handle Grafana password separately (second occurrence)
sed -i.tmp "0,/CHANGE_TO_SECURE_PASSWORD/! s/CHANGE_TO_SECURE_PASSWORD/$GRAFANA_PASSWORD/" "$ENV_FILE"

# Remove temp file
rm -f "${ENV_FILE}.tmp"

log_info "$ENV_FILE created successfully!"
echo ""
echo -e "${BLUE}============================================${NC}"
echo -e "${GREEN}Generated Credentials (SAVE THESE!)${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""
echo -e "${YELLOW}PostgreSQL:${NC}"
echo "  User: fastembed"
echo "  Password: $POSTGRES_PASSWORD"
echo "  Database: fastembed"
echo ""
echo -e "${YELLOW}Grafana:${NC}"
echo "  User: admin"
echo "  Password: $GRAFANA_PASSWORD"
echo "  URL: http://YOUR_SERVER:3000"
echo ""
echo -e "${YELLOW}API Secret Key:${NC}"
echo "  $SECRET_KEY"
echo ""
echo -e "${BLUE}============================================${NC}"
echo ""
log_info "Next steps:"
echo "  1. Review and customize .env if needed"
echo "  2. Deploy: make deploy"
echo "  3. Create API keys: make create-api-key"
echo ""
log_warn "IMPORTANT: Save these credentials in a secure password manager!"
echo ""
