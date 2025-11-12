# Quick Load Test Guide

Fast, configurable load testing with k6 for rapid iteration and testing.

## Prerequisites

Install k6:
```bash
# macOS
brew install k6

# Ubuntu/Debian
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Or visit: https://grafana.com/docs/k6/latest/set-up/install-k6/
```

## Usage

### Basic Usage

```bash
# Default: 100 requests, 1 virtual user (sequential)
make quick-test
```

### Custom Configuration

```bash
# Custom number of requests
make quick-test NUM_REQUESTS=200

# Custom number of virtual users (concurrent users)
make quick-test NUM_USERS=10

# Combine options
make quick-test NUM_REQUESTS=500 NUM_USERS=10

# Custom API key
make quick-test API_KEY=your_actual_key

# Custom text query
make quick-test TEXT="custom search query"

# Custom endpoint
make quick-test API_URL=http://production.api.com/v1/embed

# All together
make quick-test NUM_REQUESTS=1000 NUM_USERS=20 API_KEY=my_key TEXT="test"
```

### Direct Script Usage

```bash
# Using k6 directly (must include --summary-trend-stats for percentiles)
NUM_REQUESTS=100 NUM_USERS=1 \
  k6 run --summary-trend-stats="min,avg,med,max,p(90),p(95),p(99)" \
  scripts/performance/quick_test.js

# With all options
NUM_REQUESTS=500 \
  NUM_USERS=10 \
  API_KEY=your_key \
  TEXT="search query" \
  API_URL=http://localhost:8000/v1/embed \
  k6 run --summary-trend-stats="min,avg,med,max,p(90),p(95),p(99)" \
  scripts/performance/quick_test.js
```

## Example Output

```
FastEmbed Quick Load Test (k6)
================================
Configuration:
  API URL:        http://localhost:8000/v1/embed
  Total Requests: 100
  Virtual Users:  1
  Text:           "how to reset password"

Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Time:     10.52s
Successful:     100/100 (100.0%)
RPS:            9.51 req/s

Latency (ms):
  Min:          0.89ms
  Avg:          5.23ms
  p50:          4.81ms
  p95:          8.45ms
  p99:          12.34ms
  Max:          15.67ms

Cache Hit Rate: 23.0% (23/100)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Tip: Customize with environment variables:
  make quick-test NUM_REQUESTS=200 NUM_USERS=10
  API_KEY=your_key TEXT="custom query" make quick-test
```

## What It Measures

| Metric | Description |
|--------|-------------|
| **RPS** | Requests per second (throughput) |
| **Min** | Fastest request latency |
| **Avg** | Average latency across all requests |
| **p50** | Median latency (50% of requests faster) |
| **p95** | 95th percentile (95% of requests faster) |
| **p99** | 99th percentile (99% of requests faster) |
| **Max** | Slowest request latency |
| **Cache Hit Rate** | % of requests served from cache |
| **Virtual Users** | Number of concurrent users |

## Common Scenarios

### 1. Quick Sanity Check (Sequential)

```bash
# After making code changes
make quick-test
```

### 2. Test Concurrency

```bash
# 10 concurrent users
make quick-test NUM_USERS=10

# 50 concurrent users, 1000 requests total
make quick-test NUM_REQUESTS=1000 NUM_USERS=50
```

### 3. Compare Before/After

```bash
# Before changes
make quick-test > results_before.txt

# Make your changes...

# After changes
make quick-test > results_after.txt

# Compare
diff results_before.txt results_after.txt
```

### 4. Test Cache Warming

```bash
# First run (cold cache)
make quick-test

# Second run (warm cache)
make quick-test  # Should show higher cache hit rate
```

### 5. Test Different Query Sizes

```bash
# Short query
make quick-test TEXT="hello"

# Medium query
make quick-test TEXT="how to reset my password"

# Long query
make quick-test TEXT="how to reset my password and recover my account if I forgot my email"
```

### 6. Stress Test

```bash
# More requests
make quick-test NUM_REQUESTS=1000

# High concurrency
make quick-test NUM_REQUESTS=5000 NUM_USERS=100
```

## Performance Targets

Based on typical hardware (M1/M2 or modern x86_64):

### Sequential (NUM_USERS=1)

| Metric | Target | Good | Excellent |
|--------|--------|------|-----------|
| **RPS** | >10 | >50 | >100 |
| **Avg Latency** | <20ms | <10ms | <5ms |
| **p95 Latency** | <50ms | <20ms | <10ms |
| **p99 Latency** | <100ms | <40ms | <20ms |
| **Cache Hit (2nd run)** | >50% | >80% | >95% |

### Concurrent (NUM_USERS=10)

| Metric | Target | Good | Excellent |
|--------|--------|------|-----------|
| **RPS** | >50 | >200 | >500 |
| **Avg Latency** | <100ms | <50ms | <20ms |
| **p95 Latency** | <200ms | <100ms | <50ms |
| **p99 Latency** | <500ms | <200ms | <100ms |

## Interpreting Results

### High Latency

If latencies are higher than expected:

1. **Check CPU usage** - Server might be overloaded
2. **Check model loading** - First request is slower
3. **Check database** - Connection pool exhausted?
4. **Check Redis** - Cache connection issues?
5. **Check concurrency** - Too many virtual users?

### Low RPS

If RPS is lower than expected:

1. **Check NUM_USERS** - Sequential (1 VU) is naturally slower
2. **Increase concurrency** - Try NUM_USERS=10 or higher
3. **Check server capacity** - Monitor CPU/memory usage
4. **Network latency** - Test on same machine to isolate

### High p99 vs Avg

If p99 is much higher than average:

1. **Occasional slow requests** - Could be GC, cache misses, etc.
2. **Cold starts** - First few requests are slower
3. **Resource contention** - Intermittent CPU/memory spikes
4. **Connection pool** - Database connections being created

## Troubleshooting

### "Server might not be running"

```bash
# Start the server first
make run
```

### "k6 is not installed"

```bash
# macOS
brew install k6

# Ubuntu
# See prerequisites section above
```

### "No successful requests"

Check:
1. Is API_KEY correct?
2. Is server running on port 8000?
3. Try manual request:
   ```bash
   curl -X POST http://localhost:8000/v1/embed \
     -H "Authorization: Bearer your_key" \
     -H "Content-Type: application/json" \
     -d '{"text": "test"}'
   ```

## Comparison: Sequential vs Concurrent

### Sequential (NUM_USERS=1)

- **Use when**: Testing single-user performance, measuring latency accurately
- **Advantage**: Pure latency measurement, no concurrency effects
- **RPS**: Limited by sequential execution (~10-100 req/s)

### Concurrent (NUM_USERS=10+)

- **Use when**: Testing server under load, measuring throughput
- **Advantage**: Higher RPS, tests concurrency handling
- **RPS**: Much higher (100-1000+ req/s depending on hardware)
- **Note**: Latency will be higher due to contention

## Integration with CI/CD

```yaml
# .github/workflows/performance.yml
- name: Quick performance test
  run: |
    make run &
    sleep 5
    make quick-test NUM_REQUESTS=50

    # Fail if thresholds not met
    if [ $? -ne 0 ]; then
      echo "Performance regression detected!"
      exit 1
    fi
```

## Comparison with Other Tools

| Tool | Concurrency | Setup | Metrics | Use Case |
|------|-------------|-------|---------|----------|
| **quick-test (k6)** | ✅ Configurable | Easy | Excellent | Daily testing |
| **wrk** | ✅ High | Medium | Good | HTTP load |
| **k6 (full)** | ✅ Scenarios | Complex | Excellent | Comprehensive |

**Use quick-test when:**
- ✅ Testing after code changes
- ✅ Quick sanity checks
- ✅ Comparing before/after
- ✅ Need concurrency testing
- ✅ Want built-in thresholds

**Use full k6/wrk tests when:**
- ✅ Need complex scenarios
- ✅ Long-duration tests
- ✅ Production-like testing
- ✅ Detailed reporting

## Tips

1. **Run twice** - First run warms up caches
2. **Watch logs** - Keep server logs visible: `make run`
3. **Consistent queries** - Use same TEXT for fair comparisons
4. **Start sequential** - Test with NUM_USERS=1 first, then increase
5. **Monitor resources** - Watch CPU/memory during tests
6. **Document** - Save baseline results for comparison
7. **CI/CD integration** - Add to automated testing pipeline

## Advanced: Custom k6 Script

To add thresholds, custom metrics, or modify behavior, edit:
```
scripts/performance/quick_test.js
```

### Adding Performance Thresholds

By default, quick-test doesn't fail on slow performance - it just reports metrics. To add thresholds:

```javascript
export const options = {
  vus: NUM_USERS,
  iterations: NUM_REQUESTS,

  thresholds: {
    http_req_failed: ['rate<0.01'],  // Less than 1% errors
    http_req_duration: ['p(95)<20', 'p(99)<50'],  // Fail if too slow
    checks: ['rate>0.99'],  // 99% of checks should pass
  },
};
```

With thresholds, k6 will exit with error code 99 if any threshold fails.

### Custom Metrics

```javascript
import { Trend } from 'k6/metrics';
const myMetric = new Trend('my_custom_metric');

export default function () {
  // ... your test code
  myMetric.add(someValue);
}
```

## See Also

- [PERFORMANCE.md](PERFORMANCE.md) - Full performance testing guide
- [LOAD_TESTING.md](LOAD_TESTING.md) - Detailed load testing guide
- Full k6 tests - `make load-test-k6`
- wrk tests - `make load-test-wrk`
- Criterion benchmarks - `make bench`
