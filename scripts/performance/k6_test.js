// Load testing with k6
// Install: brew install k6 (macOS) or https://k6.io/docs/getting-started/installation/
// Run: k6 run scripts/performance/k6_test.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const embeddingLatency = new Trend('embedding_latency', true);

// Configuration
const API_URL = __ENV.API_URL || 'http://localhost:8000/v1/embed';
const API_KEY = __ENV.API_KEY || 'fe_test_key_here';

// Test scenarios
export const options = {
  scenarios: {
    // Scenario 1: Constant load
    constant_load: {
      executor: 'constant-vus',
      vus: 10,
      duration: '30s',
      tags: { scenario: 'constant' },
      startTime: '0s',
    },
    // Scenario 2: Ramping load
    ramping_load: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 20 },
        { duration: '1m', target: 20 },
        { duration: '30s', target: 50 },
        { duration: '1m', target: 50 },
        { duration: '30s', target: 0 },
      ],
      tags: { scenario: 'ramping' },
      startTime: '35s',
    },
    // Scenario 3: Spike test
    spike_test: {
      executor: 'constant-vus',
      vus: 100,
      duration: '10s',
      tags: { scenario: 'spike' },
      startTime: '4m30s',
    },
  },
  thresholds: {
    // Define pass/fail criteria
    'http_req_duration': ['p(95)<50', 'p(99)<100'], // 95% under 50ms, 99% under 100ms
    'errors': ['rate<0.1'], // Error rate should be less than 10%
    'http_req_failed': ['rate<0.05'], // Failed requests less than 5%
  },
};

// Test data - various query types
const queries = [
  'how to reset password',
  'customer support contact',
  'technical documentation',
  'product pricing information',
  'account settings',
  'billing and invoices',
  'security best practices',
  'api integration guide',
  'troubleshooting common issues',
  'getting started tutorial',
];

export default function () {
  // Select random query
  const query = queries[Math.floor(Math.random() * queries.length)];

  const payload = JSON.stringify({
    text: query,
    normalize: true,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
    tags: { name: 'embedding_request' },
  };

  const response = http.post(API_URL, payload, params);

  // Check response
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'has embedding': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.embedding && body.embedding.length === 384;
      } catch (e) {
        return false;
      }
    },
    'has model': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.model !== undefined;
      } catch (e) {
        return false;
      }
    },
    'latency < 100ms': (r) => r.timings.duration < 100,
  });

  // Record error rate
  errorRate.add(!success);

  // Parse response body
  if (response.status === 200) {
    try {
      const body = JSON.parse(response.body);

      // Track cache hits
      if (body.cached !== undefined) {
        cacheHitRate.add(body.cached);
      }

      // Track latency
      if (body.latency_ms !== undefined) {
        embeddingLatency.add(body.latency_ms);
      }
    } catch (e) {
      console.error('Failed to parse response:', e);
    }
  }

  // Think time - simulate real user behavior
  sleep(0.1);
}

export function handleSummary(data) {
  // Custom summary output
  console.log('\n=== Performance Test Summary ===');
  console.log(`Total Requests: ${data.metrics.http_reqs.values.count}`);
  console.log(`Request Rate: ${data.metrics.http_reqs.values.rate.toFixed(2)} req/s`);
  console.log(`Failed Requests: ${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%`);
  console.log(`Error Rate: ${(data.metrics.errors.values.rate * 100).toFixed(2)}%`);
  console.log('\nLatency:');
  console.log(`  p50: ${data.metrics.http_req_duration.values['p(50)'].toFixed(2)}ms`);
  console.log(`  p95: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms`);
  console.log(`  p99: ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms`);
  console.log(`  max: ${data.metrics.http_req_duration.values.max.toFixed(2)}ms`);

  if (data.metrics.cache_hits) {
    console.log(`\nCache Hit Rate: ${(data.metrics.cache_hits.values.rate * 100).toFixed(2)}%`);
  }

  if (data.metrics.embedding_latency) {
    console.log('\nEmbedding Latency (from API):');
    console.log(`  p50: ${data.metrics.embedding_latency.values['p(50)'].toFixed(2)}ms`);
    console.log(`  p95: ${data.metrics.embedding_latency.values['p(95)'].toFixed(2)}ms`);
    console.log(`  p99: ${data.metrics.embedding_latency.values['p(99)'].toFixed(2)}ms`);
  }

  console.log('\n===============================\n');

  return {
    'stdout': JSON.stringify(data, null, 2),
    'summary.json': JSON.stringify(data),
  };
}
