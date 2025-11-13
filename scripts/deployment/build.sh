#!/bin/bash
set -euo pipefail

# Smally API - Local Build Script
# Cross-compiles ARM64 Linux binaries locally in Docker

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

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

cd "$PROJECT_ROOT"

# Capture git info
log_info "Capturing build information..."
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_DATE=$(git log -1 --format=%ci 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git status --porcelain 2>/dev/null | wc -l | awk '{if ($1 > 0) print "true"; else print "false"}')
BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
RUST_VERSION="1.91.0"

log_info "Git Hash: $GIT_HASH"
log_info "Git Branch: $GIT_BRANCH"
log_info "Build Timestamp: $BUILD_TIMESTAMP"

# Build in Docker (ARM64 Linux) with BuildKit for cache mounts
log_info "Building binaries in Docker with BuildKit cache..."
DOCKER_BUILDKIT=1 docker build \
  --platform linux/arm64 \
  --target builder \
  --build-arg GIT_HASH="$GIT_HASH" \
  --build-arg GIT_BRANCH="$GIT_BRANCH" \
  --build-arg GIT_DATE="$GIT_DATE" \
  --build-arg GIT_DIRTY="$GIT_DIRTY" \
  --build-arg BUILD_TIMESTAMP="$BUILD_TIMESTAMP" \
  --build-arg RUST_VERSION="$RUST_VERSION" \
  -t smally-builder:latest \
  -f Dockerfile \
  .

# Extract binaries from builder image
log_info "Extracting binaries..."
mkdir -p dist
docker create --name smally-builder-temp smally-builder:latest
docker cp smally-builder-temp:/build/target/release/api dist/api
docker cp smally-builder-temp:/build/target/release/create_api_key dist/create_api_key
docker rm smally-builder-temp

# Make binaries executable
chmod +x dist/api dist/create_api_key

log_info "Build complete!"
log_info "Binaries:"
ls -lh dist/

log_info ""
log_info "=========================================="
log_info "✅ Build complete!"
log_info "=========================================="
log_info ""
log_info "Binaries are ready in: dist/"
log_info "- dist/api ($(du -h dist/api | cut -f1))"
log_info "- dist/create_api_key ($(du -h dist/create_api_key | cut -f1))"
log_info ""
log_info "To deploy manually:"
log_info ""
log_info "1. Upload binaries:"
log_info "   scp dist/api dist/create_api_key user@server:/home/smally/smally-api/"
log_info ""
log_info "2. SSH to server and restart:"
log_info "   ssh user@server"
log_info "   sudo systemctl stop smally"
log_info "   sudo systemctl start smally"
log_info "   sudo systemctl status smally"
log_info ""
log_info "Or use Ansible for automated deployment:"
log_info "   ansible-playbook -i ansible/inventory/hosts.yml ansible/deploy.yml --ask-vault-pass"
log_info ""
