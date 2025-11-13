#!/bin/bash
# Run all performance benchmarks
set -euo pipefail

# Color output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Smally API - Performance Benchmarks${NC}"
echo "========================================"
echo ""

# 1. Criterion Benchmarks (Unit-level)
echo -e "${GREEN}Running Criterion Benchmarks...${NC}"
echo "These test individual components (cache, tokenizer, inference)"
echo ""

if [ -d "models/all-MiniLM-L6-v2-onnx" ]; then
    echo "✓ Model found, running all benchmarks"
    cargo bench --bench cache_bench
    cargo bench --bench tokenizer_bench
    cargo bench --bench inference_bench
else
    echo "⚠ Model not found, running only cache benchmarks"
    echo "  Run 'make model' to download the model for full benchmarks"
    cargo bench --bench cache_bench
fi

echo ""
echo -e "${YELLOW}Criterion results saved to: target/criterion/${NC}"
echo "Open target/criterion/report/index.html to view detailed results"
echo ""

# 2. Check if server is running for load tests
echo -e "${GREEN}Checking for running server...${NC}"
if curl -s http://localhost:8000/health > /dev/null 2>&1; then
    echo "✓ Server is running on http://localhost:8000"
    echo ""

    read -p "Run load tests with wrk? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if command -v wrk &> /dev/null; then
            ./scripts/performance/wrk_test.sh
        else
            echo "⚠ wrk not installed. Install with: brew install wrk"
        fi
    fi

    echo ""
    read -p "Run load tests with k6? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if command -v k6 &> /dev/null; then
            k6 run scripts/performance/k6_test.js
        else
            echo "⚠ k6 not installed. Install with: brew install k6"
        fi
    fi
else
    echo "✗ Server is not running"
    echo "  Start server with: make run"
    echo "  Then run load tests separately:"
    echo "    - wrk: ./scripts/performance/wrk_test.sh"
    echo "    - k6:  k6 run scripts/performance/k6_test.js"
fi

echo ""
echo -e "${GREEN}Benchmark run complete!${NC}"
