# Performance Testing Guide

This document describes the performance testing setup for the Smally API and how to run various benchmarks.

## Overview

The Smally API includes three types of performance tests:

1. **Criterion Benchmarks** - Micro-benchmarks for individual components
3. **k6 Load Tests** - Advanced load testing with scenarios and metrics

## Prerequisites

### For Criterion Benchmarks

- Rust toolchain (already installed)
- Model files (download with `make model`)

### For Load Tests

- Running API server
- k6: `brew install k6` (macOS) or see [k6.io](https://k6.io/docs/getting-started/installation/)

## Quick Start

Run all benchmarks with:

```bash
./scripts/performance/run_benchmarks.sh
```

## 1. Criterion Benchmarks

Criterion provides detailed statistical analysis of component performance.

### Cache Benchmarks

Tests the LRU cache implementation:

```bash
cargo bench --bench cache_bench
```

**Tests:**

- `lru_put` - Insert operations at different cache sizes
- `lru_get_hit` - Cache hit performance
- `lru_get_miss` - Cache miss performance
- `lru_mixed_workload` - Realistic read/write mix (70% reads, 30% writes)

**Expected Results:**

- Put: ~50-100ns per operation
- Get (hit): ~20-50ns per operation
- Get (miss): ~10-20ns per operation

### Tokenizer Benchmarks

Tests text tokenization performance:

```bash
cargo bench --bench tokenizer_bench
```

**Tests:**

- Short text (5 tokens): "how to reset password"
- Medium text (20 tokens): Multi-word queries
- Long text (50 tokens): Full sentences

**Expected Results:**

- Short (5 tokens): ~50-100µs
- Medium (20 tokens): ~150-300µs
- Long (50 tokens): ~300-600µs

### Inference Benchmarks

Tests ONNX model inference:

```bash
cargo bench --bench inference_bench
```

**Tests:**

- Embedding generation for different text lengths
- Batch processing (1, 5, 10 texts)
- Normalization impact comparison

**Expected Results:**

- Short text: ~2-4ms
- Medium text: ~4-8ms
- Long text: ~8-15ms
- Normalization overhead: ~0.1-0.3ms

### Viewing Results

Criterion generates HTML reports:

```bash
open target/criterion/report/index.html
```

Results include:

- Mean execution time with confidence intervals
- Throughput measurements
- Performance comparison across runs
- Statistical analysis (outliers, variance)

## 2. k6 Load Tests

k6 provides advanced load testing with scenarios, metrics, and thresholds.

### Running k6 Tests

```bash
# Make sure server is running
make run

# In another terminal:
k6 run scripts/performance/k6_test.js
```

### Test Scenarios

The k6 script includes three scenarios:

1. **Constant Load** (30s)
   - 10 virtual users
   - Steady state performance

2. **Ramping Load** (3.5 minutes)
   - Ramps from 0 → 20 → 50 users
   - Tests scalability

3. **Spike Test** (10s)
   - 100 concurrent users
   - Tests peak capacity

### Custom k6 Test

```bash
# Quick test (10 VUs, 30 seconds)
k6 run --vus 10 --duration 30s scripts/performance/k6_test.js

# Custom scenarios
k6 run --vus 50 --duration 1m scripts/performance/k6_test.js

# With environment variables
API_KEY=your_key k6 run scripts/performance/k6_test.js
```

### Thresholds

The k6 script enforces these thresholds:

- 95% of requests < 50ms
- 99% of requests < 100ms
- Error rate < 10%
- Failed requests < 5%

**Note:** Test fails if thresholds are not met.

### Metrics

k6 tracks:

- HTTP metrics (duration, failures, etc.)
- Custom metrics:
  - Cache hit rate
  - Embedding latency (from API response)
  - Error rate

## Performance Optimization Tips

### 1. Cache Optimization

- **L1 Cache Size**: Adjust `L1_CACHE_SIZE` in .env
  - Default: 10,000 entries
  - Increase for higher cache hit rates
  - Each entry: ~1.5KB (384 floats)

- **L2 Cache (Redis)**:
  - Use Redis on same machine for minimal latency
  - Consider Redis Cluster for scale
  - Monitor Redis memory usage

### 2. Database Connection Pool

Adjust PostgreSQL pool settings:

```env
DATABASE_MAX_CONNECTIONS=20  # Increase for high concurrency
```

### 3. ONNX Runtime

- Use CPU with AVX2/AVX512 for best performance
- Consider GPU for inference if available
- Adjust thread count: `ORT_NUM_THREADS=4`

### 4. Rust Build Optimizations

Already configured in Cargo.toml:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### 5. System Tuning

**macOS:**

```bash
# Increase file descriptors
ulimit -n 10000
```

**Linux:**

```bash
# Increase file descriptors
ulimit -n 65535

# TCP tuning
sysctl -w net.core.somaxconn=4096
sysctl -w net.ipv4.tcp_max_syn_backlog=4096
```

## Continuous Performance Testing

### CI/CD Integration

Add to CI pipeline:

```yaml
- name: Run benchmarks
  run: |
    cargo bench --bench cache_bench
    # Compare with baseline
```

### Baseline Tracking

Store baseline results:

```bash
# Save baseline
cargo bench --bench cache_bench -- --save-baseline main

# Compare with baseline
cargo bench --bench cache_bench -- --baseline main
```

### Regression Detection

Use criterion's comparison features:

```bash
# This will fail if performance regresses significantly
cargo bench -- --baseline main --significance-level 0.05
```

## Troubleshooting

### Low Throughput

1. Check CPU usage: `top` or `htop`
2. Check if cache is working: `/metrics` endpoint
3. Check database connection pool exhaustion
4. Monitor Redis latency

### High Latency

1. Check p95/p99 metrics separately from mean
2. Profile with criterion flamegraphs:

   ```bash
   cargo bench --bench inference_bench -- --profile-time=5
   ```

3. Check for GC pauses (shouldn't happen in Rust)
4. Check disk I/O if model is being loaded

### Memory Issues

1. Monitor with `ps` or `htop`
2. Check L1 cache size vs available memory
3. Check for memory leaks with valgrind:

   ```bash
   valgrind --leak-check=full ./target/release/api
   ```

### Inconsistent Results

1. Run longer benchmarks (increase duration)
2. Reduce background processes
3. Pin CPU affinity:

   ```bash
   taskset -c 0-3 cargo bench
   ```

4. Disable CPU frequency scaling

## Expected Performance

Based on ARM64 (M1/M2) and x86_64 testing:

### Component Benchmarks

| Component | Operation | Time |
|-----------|-----------|------|
| LRU Cache | Get (hit) | 20-50ns |
| LRU Cache | Put | 50-100ns |
| Tokenizer | Short (5 tok) | 50-100µs |
| Tokenizer | Medium (20 tok) | 150-300µs |
| ONNX Inference | Short text | 2-4ms |
| ONNX Inference | Medium text | 4-8ms |

### End-to-End API

| Scenario | Throughput | p95 Latency | p99 Latency |
|----------|------------|-------------|-------------|
| Cached | 5000+ req/s | <1ms | <2ms |
| Uncached (light) | 200-400 req/s | 6-10ms | 12-20ms |
| Uncached (heavy) | 800-1500 req/s | 15-25ms | 30-50ms |

**Note:** Performance varies by hardware, load, and cache hit rate.

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [k6 Documentation](https://k6.io/docs/)
- [ONNX Runtime Performance Tuning](https://onnxruntime.ai/docs/performance/)
