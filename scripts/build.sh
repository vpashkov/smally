#!/bin/bash
set -euo pipefail

# Cross-compile for Linux ARM64
echo "Building for Linux ARM64..."
cargo build --release --target aarch64-unknown-linux-gnu

echo "✓ Build complete: target/aarch64-unknown-linux-gnu/release/embed_rs"
