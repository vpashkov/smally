# Build stage
# Explicitly target linux/arm64 (AWS Graviton, Apple Silicon)
FROM --platform=linux/arm64 rust:1.91-bookworm AS builder

# Build arguments for git info (passed from host)
ARG GIT_HASH=unknown
ARG GIT_BRANCH=unknown
ARG GIT_DATE=unknown
ARG GIT_DIRTY=false
ARG BUILD_TIMESTAMP=unknown
ARG RUST_VERSION=unknown

# Install build dependencies including ONNX Runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    wget \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install ONNX Runtime for build (ARM64)
RUN wget -q https://github.com/microsoft/onnxruntime/releases/download/v1.16.3/onnxruntime-linux-aarch64-1.16.3.tgz && \
    tar -xzf onnxruntime-linux-aarch64-1.16.3.tgz && \
    cp onnxruntime-linux-aarch64-1.16.3/lib/* /usr/local/lib/ && \
    cp -r onnxruntime-linux-aarch64-1.16.3/include/* /usr/local/include/ && \
    rm -rf onnxruntime-linux-aarch64-1.16.3* && \
    ldconfig

# Set environment variable for ONNX Runtime
ENV ORT_DYLIB_PATH=/usr/local/lib

WORKDIR /build

# Copy Cargo files and download dependencies first (better caching)
COPY Cargo.toml Cargo.lock build.rs ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    mkdir -p src/bin benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    echo "fn main() {}" > src/bin/create_token.rs && \
    echo "fn main() {}" > src/bin/generate_keypair.rs && \
    echo "fn main() {}" > build.rs && \
    echo "fn main() {}" > benches/cache_bench.rs && \
    echo "fn main() {}" > benches/tokenizer_bench.rs && \
    echo "fn main() {}" > benches/inference_bench.rs && \
    cargo build --release --bins && \
    rm -rf src benches build.rs

# Copy source code
COPY . .

# Build the application
# SQLX_OFFLINE=true to use cached queries
# Pass build args as env vars for build.rs to use
ENV SQLX_OFFLINE=true \
    GIT_HASH=${GIT_HASH} \
    GIT_BRANCH=${GIT_BRANCH} \
    GIT_DATE=${GIT_DATE} \
    GIT_DIRTY=${GIT_DIRTY} \
    BUILD_TIMESTAMP=${BUILD_TIMESTAMP} \
    RUST_VERSION=${RUST_VERSION}

# Use cache mounts for cargo registry, git cache, and incremental builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release && \
    cp /build/target/release/api /build/api && \
    cp /build/target/release/create_token /build/create_token && \
    cp /build/target/release/generate_keypair /build/generate_keypair

# Runtime stage
FROM --platform=linux/arm64 ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    bash \
    postgresql-client \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install ONNX Runtime (ARM64)
RUN wget -q https://github.com/microsoft/onnxruntime/releases/download/v1.16.3/onnxruntime-linux-aarch64-1.16.3.tgz && \
    tar -xzf onnxruntime-linux-aarch64-1.16.3.tgz && \
    cp onnxruntime-linux-aarch64-1.16.3/lib/* /usr/local/lib/ && \
    rm -rf onnxruntime-linux-aarch64-1.16.3* && \
    ldconfig

WORKDIR /app

# Copy the built binaries from builder stage
COPY --from=builder /build/api /app/api
COPY --from=builder /build/create_token /app/create_token
COPY --from=builder /build/generate_keypair /app/generate_keypair

# Copy scripts
COPY scripts /app/scripts

# Create directories for models and logs
RUN mkdir -p /app/models /app/logs && \
    chmod +x /app/api && \
    chmod +x /app/create_token && \
    chmod +x /app/generate_keypair && \
    chmod +x /app/scripts/*.sh 2>/dev/null || true && \
    chmod +x /app/scripts/deployment/*.sh 2>/dev/null || true

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run the application
CMD ["/app/api"]
