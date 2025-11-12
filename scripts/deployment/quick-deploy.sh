#!/bin/bash
set -euo pipefail

# FastEmbed API Quick Deploy
# For code-only changes - skips full rebuild and infrastructure setup

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

cd "$PROJECT_ROOT"

ENV_FILE=".env"

log_info "Quick deploy starting..."
log_info "Using environment file: $ENV_FILE"

# Pull latest code
log_info "Pulling latest code from git..."
git pull

# Only rebuild app container (no --no-cache for speed)
log_info "Building application (cached layers)..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" build app

# Rolling restart - minimal downtime
log_info "Restarting application..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" up -d app

log_info "Restarting nginx..."
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" restart nginx

log_info "Quick deploy complete!"
log_info "Check status: docker-compose -f docker-compose.prod.yml ps"

# Show service status
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" ps
