#!/bin/bash
set -euo pipefail

# Smally API - Native Build Script
# Builds binaries natively using local Rust toolchain (no Docker)
# Note: This produces binaries for your current platform (macOS ARM64)
# For production ARM64 Linux, use build.sh or build-no-buildkit.sh instead

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

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
  log_error "Cargo not found. Install Rust from https://rustup.rs/"
  exit 1
fi

log_info "Building natively with Rust $(rustc --version | cut -d' ' -f2)..."
log_warn "Note: This builds for $(rustc -vV | grep host | cut -d' ' -f2)"
log_warn "For production ARM64 Linux, use build.sh instead"

# Capture git info and set as env vars
export GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
export GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
export GIT_DATE=$(git log -1 --format=%ci 2>/dev/null || echo "unknown")
export GIT_DIRTY=$(git status --porcelain 2>/dev/null | wc -l | awk '{if ($1 > 0) print "true"; else print "false"}')
export BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
export RUST_VERSION=$(rustc --version | cut -d' ' -f2)

log_info "Git Hash: $GIT_HASH"
log_info "Git Branch: $GIT_BRANCH"

# Set SQLX offline mode
export SQLX_OFFLINE=true

# Build release binaries
log_info "Running cargo build --release..."
cargo build --release

# Copy to dist directory
log_info "Copying binaries to dist/..."
mkdir -p dist
cp target/release/api dist/api
cp target/release/create_token dist/create_token
cp target/release/generate_keypair dist/generate_keypair

log_info "Build complete!"
log_info "Binaries:"
ls -lh dist/

log_info ""
log_info "=========================================="
log_info "✅ Native build complete!"
log_info "=========================================="
log_info ""
log_info "Binaries are ready in: dist/"
log_info "- dist/api ($(du -h dist/api | cut -f1))"
log_info "- dist/create_token ($(du -h dist/create_token | cut -f1))"
log_info "- dist/generate_keypair ($(du -h dist/generate_keypair | cut -f1))"
log_info ""
log_info "Platform: $(rustc -vV | grep host | cut -d' ' -f2)"
log_info ""
log_warn "⚠️  These binaries are for your local platform (macOS)!"
log_warn "⚠️  For production Linux deployment, use:"
log_warn "    ./scripts/deployment/build.sh"
log_info ""
log_info "Use these binaries for:"
log_info "  - Local testing: ./dist/api"
log_info "  - Development"
log_info "  - Benchmarking on your machine"
log_info ""
