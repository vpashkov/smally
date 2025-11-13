# Load Testing Quick Reference

## All Load Testing Options

### 1. k6 Tests

```bash
# Using make (checks if server is running)
make load-test

# Direct command
k6 run scripts/performance/k6_test.js

# With custom settings
API_KEY=your_key k6 run scripts/performance/k6_test.js
API_URL=http://localhost:8000/v1/embed k6 run scripts/performance/k6_test.js

# Quick test (fewer VUs)
k6 run --vus 10 --duration 30s scripts/performance/k6_test.js
```

### Install k6

**macOS:**

```bash
brew install k6
```

**Ubuntu/Debian:**

```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Other:** See [k6.io/docs/getting-started/installation](https://k6.io/docs/getting-started/installation/)

## k6 Test Details

### What k6 Tests

The k6 script includes three scenarios:

1. **Constant Load** (30s)
   - 10 virtual users
   - Steady state testing

2. **Ramping Load** (3.5 minutes)
   - Ramps: 0 → 20 → 50 users
   - Tests scalability

3. **Spike Test** (10s)
   - 100 concurrent users
   - Tests peak capacity

### Performance Thresholds

Tests fail if:

- 95% of requests > 50ms
- 99% of requests > 100ms
- Error rate > 10%
- Failed requests > 5%

### Custom Metrics Tracked

- Cache hit rate
- Embedding latency (from API)
- Error rate by type

## Customizing k6 Tests

### Modify Scenarios

Edit `scripts/performance/k6_test.js`:

```javascript
export const options = {
  scenarios: {
    my_scenario: {
      executor: 'constant-vus',
      vus: 20,              // Change virtual users
      duration: '1m',       // Change duration
    },
  },
};
```

### Change Thresholds

```javascript
export const options = {
  thresholds: {
    'http_req_duration': ['p(95)<100'],  // Relax threshold
  },
};
```

### Use Different Queries

```javascript
const queries = [
  'your custom query',
  'another query',
];
```

## Comparing Results

### Run Multiple Times

```bash
# Run 1
k6 run scripts/performance/k6_test.js > results/run1.txt

# Run 2
k6 run scripts/performance/k6_test.js > results/run2.txt

# Compare
diff results/run1.txt results/run2.txt
```

### Save Results as JSON

```bash
k6 run --out json=results.json scripts/performance/k6_test.js
```

### Cloud Results (k6 Cloud)

```bash
# Sign up at k6.io/cloud
k6 login cloud

# Run and upload results
k6 run --out cloud scripts/performance/k6_test.js
```

## Troubleshooting

### "Server is not running"

```bash
# Check if server is up
curl http://localhost:8000/health

# Start server
make run
```

### "k6 is not installed"

```bash
# Install k6
brew install k6  # macOS
# Or see installation guide above
```

### High Error Rates

1. **Check server logs** - Look for errors
2. **Reduce VUs** - Start with fewer users
3. **Check resources** - CPU/memory usage
4. **Verify API key** - Set correct `API_KEY` env var

### Inconsistent Results

1. **Warm up first** - Run once, then test again
2. **Close other apps** - Reduce background load
3. **Increase duration** - Longer tests are more stable
4. **Check network** - Ensure stable connection

## Advanced Usage

### Test Against Production

```bash
API_URL=https://your-production-api.com/v1/embed \
API_KEY=prod_key \
k6 run scripts/performance/k6_test.js
```

### Multiple Endpoints

Edit script to test multiple endpoints:

```javascript
const endpoints = [
  '/v1/embed',
  '/v1/batch-embed',
];

export default function() {
  const endpoint = endpoints[Math.floor(Math.random() * endpoints.length)];
  http.post(API_URL + endpoint, payload, params);
}
```

### Distributed Testing

Use k6 cloud for distributed load:

```bash
k6 cloud scripts/performance/k6_test.js
```

## Example Output

```
     ✓ status is 200
     ✓ has embedding
     ✓ has model
     ✓ latency < 100ms

     checks.........................: 100.00% ✓ 12000 ✗ 0
     data_received..................: 2.1 MB  71 kB/s
     data_sent......................: 360 kB  12 kB/s
     http_req_blocked...............: avg=1.2ms   p(95)=5.4ms
     http_req_duration..............: avg=8.5ms   p(95)=15.2ms p(99)=28.1ms
     http_reqs......................: 3000    100/s

=== Performance Test Summary ===
Total Requests: 3000
Request Rate: 100.00 req/s
Failed Requests: 0.00%
Error Rate: 0.00%

Latency:
  p50: 7.23ms
  p95: 15.18ms
  p99: 28.11ms
  max: 45.67ms

Cache Hit Rate: 15.43%

Embedding Latency (from API):
  p50: 6.45ms
  p95: 13.21ms
  p99: 25.89ms
```

## Quick Reference Card

| Command | Description |
|---------|-------------|
| `make load-test` | Run load tests |
| `k6 run <script>` | Direct k6 execution |
| `k6 run --vus N --duration Xs` | Custom load |
| `k6 run --out json=file.json` | Save results |
| `API_KEY=key k6 run` | Custom API key |

## See Also

- [PERFORMANCE.md](PERFORMANCE.md) - Full performance testing guide
- [k6 Documentation](https://k6.io/docs/) - Official k6 docs
