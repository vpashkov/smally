#!/bin/bash
# Load testing with wrk (HTTP benchmarking tool)
# Install: brew install wrk (macOS) or apt-get install wrk (Ubuntu)

set -euo pipefail

API_URL="${API_URL:-http://localhost:8000/v1/embed}"
API_KEY="${API_KEY:-fe_test_key_here}"

# Color output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Smally API - Load Testing with wrk${NC}"
echo "=================================="
echo ""

# Check if wrk is installed
if ! command -v wrk &> /dev/null; then
    echo "Error: wrk is not installed"
    echo "Install with: brew install wrk (macOS) or apt-get install wrk (Ubuntu)"
    exit 1
fi

# Create temp directory for wrk script
SCRIPT_DIR=$(mktemp -d)
trap "rm -rf $SCRIPT_DIR" EXIT

# Create Lua script for POST request
cat > "$SCRIPT_DIR/post.lua" << 'EOF'
wrk.method = "POST"
wrk.body = '{"text": "how to reset password", "normalize": true}'
wrk.headers["Content-Type"] = "application/json"
wrk.headers["Authorization"] = "Bearer " .. os.getenv("API_KEY")

-- Optional: Track latency
latencies = {}

function response(status, headers, body)
    if status ~= 200 then
        print("Error: " .. status .. " - " .. body)
    end
end

function done(summary, latency, requests)
    io.write("------------------------------\n")
    io.write(string.format("  Requests:      %d\n", summary.requests))
    io.write(string.format("  Duration:      %.2fs\n", summary.duration / 1000000))
    io.write(string.format("  Req/sec:       %.2f\n", summary.requests / (summary.duration / 1000000)))
    io.write(string.format("  Latency p50:   %.2fms\n", latency:percentile(50)))
    io.write(string.format("  Latency p95:   %.2fms\n", latency:percentile(95)))
    io.write(string.format("  Latency p99:   %.2fms\n", latency:percentile(99)))
    io.write(string.format("  Latency max:   %.2fms\n", latency.max))
    io.write("------------------------------\n")
end
EOF

echo "Test Configuration:"
echo "  API URL: $API_URL"
echo "  API Key: ${API_KEY:0:10}..."
echo ""

# Test 1: Warm-up
echo -e "${GREEN}Test 1: Warm-up (10 seconds, 2 connections)${NC}"
wrk -t2 -c2 -d10s -s "$SCRIPT_DIR/post.lua" "$API_URL"
echo ""

# Test 2: Light load
echo -e "${GREEN}Test 2: Light load (30 seconds, 10 connections)${NC}"
wrk -t4 -c10 -d30s -s "$SCRIPT_DIR/post.lua" "$API_URL"
echo ""

# Test 3: Medium load
echo -e "${GREEN}Test 3: Medium load (30 seconds, 50 connections)${NC}"
wrk -t8 -c50 -d30s -s "$SCRIPT_DIR/post.lua" "$API_URL"
echo ""

# Test 4: Heavy load
echo -e "${GREEN}Test 4: Heavy load (30 seconds, 100 connections)${NC}"
wrk -t12 -c100 -d30s -s "$SCRIPT_DIR/post.lua" "$API_URL"
echo ""

# Test 5: Spike test
echo -e "${GREEN}Test 5: Spike test (10 seconds, 200 connections)${NC}"
wrk -t16 -c200 -d10s -s "$SCRIPT_DIR/post.lua" "$API_URL"
echo ""

echo -e "${GREEN}Load testing complete!${NC}"
