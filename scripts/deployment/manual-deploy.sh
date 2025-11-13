#!/bin/bash
set -euo pipefail

# Manual Binary Deployment Script
# Builds locally and deploys via SSH/SCP without Ansible

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Check arguments
if [ $# -lt 1 ]; then
  echo "Usage: $0 <user@server> [app_dir]"
  echo ""
  echo "Examples:"
  echo "  $0 root@157.180.64.40"
  echo "  $0 ubuntu@myserver.com /opt/smally"
  echo ""
  exit 1
fi

SERVER="$1"
APP_DIR="${2:-/home/smally/smally-api}"

cd "$PROJECT_ROOT"

log_info "Building binaries locally..."
./scripts/deployment/build-and-deploy.sh

if [ ! -f "dist/api" ] || [ ! -f "dist/create_api_key" ]; then
  log_error "Build failed - binaries not found in dist/"
  exit 1
fi

log_info "Stopping service on remote server..."
ssh "$SERVER" "sudo systemctl stop smally || true"

log_info "Uploading binaries to $SERVER:$APP_DIR..."
scp dist/api dist/create_api_key "$SERVER:$APP_DIR/"

log_info "Setting permissions..."
ssh "$SERVER" "chmod +x $APP_DIR/api $APP_DIR/create_api_key"

log_info "Starting service..."
ssh "$SERVER" "sudo systemctl start smally"

log_info "Waiting for service to start..."
sleep 3

log_info "Checking service status..."
ssh "$SERVER" "sudo systemctl status smally --no-pager" || true

log_info ""
log_info "=========================================="
log_info "✅ Deployment complete!"
log_info "=========================================="
log_info ""
log_info "Check health endpoint:"
log_info "  ssh $SERVER 'curl -s http://localhost:8000/health | jq .'"
log_info ""
log_info "View logs:"
log_info "  ssh $SERVER 'sudo journalctl -u smally -f'"
log_info ""
