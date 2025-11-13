# Build stage
FROM rust:1.91-bookworm AS builder

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
RUN mkdir -p src/bin benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    echo "fn main() {}" > src/bin/create_api_key.rs && \
    echo "fn main() {}" > build.rs && \
    echo "fn main() {}" > benches/cache_bench.rs && \
    echo "fn main() {}" > benches/tokenizer_bench.rs && \
    echo "fn main() {}" > benches/inference_bench.rs && \
    cargo build --release --bins && \
    rm -rf src benches build.rs

# Copy source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM ubuntu:22.04

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
COPY --from=builder /build/target/release/api /app/api
COPY --from=builder /build/target/release/create_api_key /app/create_api_key

# Copy scripts
COPY scripts /app/scripts

# Create directories for models and logs
RUN mkdir -p /app/models /app/logs && \
    chmod +x /app/api && \
    chmod +x /app/create_api_key && \
    chmod +x /app/scripts/*.sh 2>/dev/null || true && \
    chmod +x /app/scripts/deployment/*.sh 2>/dev/null || true

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run the application
CMD ["/app/api"]
