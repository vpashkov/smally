#!/bin/bash
set -euo pipefail

# FastEmbed API - Create API Key Script (Production)
# Wrapper to run init_db.py inside Docker container

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Check arguments
if [ $# -lt 1 ]; then
  log_error "Usage: $0 <email> [tier]"
  echo ""
  echo "Examples:"
  echo "  $0 user@example.com free"
  echo "  $0 customer@company.com pro"
  echo "  $0 admin@example.com scale"
  echo ""
  echo "Tiers: free (default), pro, scale"
  exit 1
fi

EMAIL="$1"
TIER="${2:-free}"

cd "$PROJECT_ROOT"

ENV_FILE=".env"

log_info "Creating API key for: $EMAIL (tier: $TIER)"
log_info "Using environment: $ENV_FILE"

# Run init_db.py inside the Docker container
docker-compose -f docker-compose.prod.yml --env-file "$ENV_FILE" exec app \
  python scripts/init_db.py "$EMAIL" "$TIER"

log_info "Done! Save the API key shown above."
