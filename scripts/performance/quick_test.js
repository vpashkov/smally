// Quick Load Test - k6 script for rapid iteration testing
// Usage: k6 run quick_test.js
// Or with Make: make quick-test NUM_REQUESTS=100 NUM_USERS=1

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Trend } from 'k6/metrics';

// Custom metrics
const cacheHits = new Counter('cache_hits');
const cacheTotal = new Counter('cache_total');

// Configuration from environment variables
const NUM_REQUESTS = parseInt(__ENV.NUM_REQUESTS || '100');
const NUM_USERS = parseInt(__ENV.NUM_USERS || '1');
const API_URL = __ENV.API_URL || 'http://localhost:8000/v1/embed';
const API_KEY = __ENV.API_KEY || 'fe_test_key_here';
const TEXT = __ENV.TEXT || 'how to reset password';

export const options = {
  vus: NUM_USERS,
  iterations: NUM_REQUESTS,

  // Note: Percentiles are calculated via --summary-trend-stats in Makefile
  // No pass/fail thresholds for quick tests - just collect metrics
};

export default function () {
  const payload = JSON.stringify({
    text: TEXT,
    normalize: true,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
  };

  const res = http.post(API_URL, payload, params);

  // Check response
  const success = check(res, {
    'status is 200': (r) => r.status === 200,
    'has embedding': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.embedding && body.embedding.length > 0;
      } catch (e) {
        return false;
      }
    },
  });

  // Track cache hits
  if (success && res.status === 200) {
    try {
      const body = JSON.parse(res.body);
      cacheTotal.add(1);
      if (body.cached === true) {
        cacheHits.add(1);
      }
    } catch (e) {
      // Ignore parse errors
    }
  }
}

export function handleSummary(data) {
  // Calculate cache hit rate
  const hits = data.metrics.cache_hits?.values?.count || 0;
  const total = data.metrics.cache_total?.values?.count || 0;
  const cacheHitRate = total > 0 ? (hits / total * 100).toFixed(1) : '0.0';

  // Get key metrics
  const reqDuration = data.metrics.http_req_duration || {};
  const values = reqDuration.values || {};

  const totalReqs = data.metrics.http_reqs?.values?.count || 0;
  const failedReqs = data.metrics.http_req_failed?.values?.count || 0;
  const successReqs = totalReqs - failedReqs;

  const totalTime = data.state?.testRunDurationMs / 1000 || 0;
  const rps = totalTime > 0 ? (totalReqs / totalTime).toFixed(2) : '0.00';

  // Build custom summary
  const summary = `
FastEmbed Quick Load Test (k6)
================================
Configuration:
  API URL:        ${API_URL}
  Total Requests: ${NUM_REQUESTS}
  Virtual Users:  ${NUM_USERS}
  Text:           "${TEXT}"

Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Time:     ${totalTime.toFixed(2)}s
Successful:     ${successReqs}/${totalReqs} (${totalReqs > 0 ? (successReqs/totalReqs*100).toFixed(1) : '0'}%)
RPS:            ${rps} req/s

Latency (ms):
  Min:          ${(values.min || 0).toFixed(2)}ms
  Avg:          ${(values.avg || 0).toFixed(2)}ms
  p50:          ${(values.med || 0).toFixed(2)}ms
  p95:          ${(values['p(95)'] || 0).toFixed(2)}ms
  p99:          ${(values['p(99)'] || 0).toFixed(2)}ms
  Max:          ${(values.max || 0).toFixed(2)}ms

Cache Hit Rate: ${cacheHitRate}% (${hits}/${total})
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Tip: Customize with environment variables:
  make quick-test NUM_REQUESTS=200 NUM_USERS=10
  API_KEY=your_key TEXT="custom query" make quick-test
`;

  return {
    'stdout': summary,
  };
}
