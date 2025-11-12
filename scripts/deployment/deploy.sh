#!/bin/bash
set -euo pipefail

# FastEmbed API Deployment Script
# This script deploys the application to production

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
  log_error "This script should not be run as root"
  exit 1
fi

# Check required commands
for cmd in docker docker-compose; do
  if ! command -v $cmd &>/dev/null; then
    log_error "$cmd is not installed"
    exit 1
  fi
done

cd "$PROJECT_ROOT"

# Check if .env exists
if [[ ! -f .env ]]; then
  log_error ".env file not found"
  log_info "Run: ./scripts/deployment/generate-env.sh"
  exit 1
fi

# Validate environment variables (check for placeholder values)
if grep -q "CHANGE_TO_SECURE_PASSWORD" .env || \
   grep -q "GENERATE_SECURE_RANDOM_KEY" .env; then
  log_error "Please run ./scripts/deployment/generate-env.sh to generate secure credentials"
  exit 1
fi

ENV_FILE=".env"

log_info "Using environment file: $ENV_FILE"

log_info "Pulling latest images..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" pull

log_info "Building application image..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" build --no-cache app

log_info "Stopping existing containers..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" down

log_info "Starting services..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" up -d

log_info "Waiting for services to be healthy..."
sleep 10

# Check service health
RETRIES=30
while [ $RETRIES -gt 0 ]; do
  if docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" ps | grep -q "unhealthy"; then
    log_warn "Waiting for services to become healthy... ($RETRIES retries left)"
    sleep 2
    RETRIES=$((RETRIES - 1))
  else
    break
  fi
done

if [ $RETRIES -eq 0 ]; then
  log_error "Services failed to become healthy"
  docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" logs
  exit 1
fi

log_info "Initializing database..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" exec -T app python scripts/init_db.py admin@example.com scale || true

log_info "Deployment complete!"
log_info ""
log_info "============================================"
log_info "API available at: https://$(hostname)/v1/embed"
log_info "Metrics: http://localhost:9090"
log_info "Grafana: http://localhost:3000"
log_info ""
log_info "Create additional API keys:"
log_info "  docker-compose -f docker-compose.prod.yml exec app python scripts/init_db.py user@example.com <tier>"
log_info "  Tiers: free, pro, scale"
log_info "============================================"

# Show service status
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" ps
