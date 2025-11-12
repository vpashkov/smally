.PHONY: help deps build run services-up services-down model init-db clean test docker-build docker-up docker-down deploy quick-deploy health-check backup create-api-key logs-prod check bench bench-cache bench-tokenizer bench-inference perf-test load-test load-test-k6 load-test-wrk quick-test

help:
	@echo "FastEmbed API (Rust) - Make Commands"
	@echo ""
	@echo "Setup:"
	@echo "  make deps          - Download Rust dependencies"
	@echo "  make services-up   - Start PostgreSQL and Redis"
	@echo "  make services-down - Stop services"
	@echo "  make model         - Download ONNX model from Hugging Face"
	@echo "  make init-db       - Initialize database and create admin user"
	@echo ""
	@echo "Building:"
	@echo "  make build         - Build the server binary (release)"
	@echo "  make check         - Check code compilation"
	@echo "  make run           - Run the API server (release mode)"
	@echo "  make dev           - Run in development mode with auto-reload"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt           - Format code with rustfmt"
	@echo "  make clippy        - Run clippy linter"
	@echo "  make test          - Run tests"
	@echo ""
	@echo "Performance:"
	@echo "  make bench         - Run all criterion benchmarks"
	@echo "  make bench-cache   - Run cache benchmarks only"
	@echo "  make bench-tokenizer - Run tokenizer benchmarks only"
	@echo "  make bench-inference - Run inference benchmarks only"
	@echo "  make quick-test    - Quick load test (k6, customizable)"
	@echo "                       Usage: make quick-test NUM_REQUESTS=100 NUM_USERS=1"
	@echo "  make load-test     - Run load tests (k6 + wrk)"
	@echo "  make load-test-k6  - Run k6 load tests only"
	@echo "  make load-test-wrk - Run wrk load tests only"
	@echo "  make perf-test     - Run full performance test suite"
	@echo ""
	@echo "Production:"
	@echo "  make docker-build  - Build production Docker image"
	@echo "  make docker-up     - Start production services"
	@echo "  make docker-down   - Stop production services"
	@echo "  make deploy        - Deploy to production (full)"
	@echo "  make quick-deploy  - Quick deploy (code changes only)"
	@echo "  make create-api-key - Create API key (production)"
	@echo "  make health-check  - Run health checks"
	@echo "  make backup        - Backup database"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean         - Clean up build artifacts"
	@echo "  make logs          - Show docker logs"
	@echo "  make logs-prod     - Show production docker logs"
	@echo ""
	@echo "Complete Setup:"
	@echo "  make setup         - Run full setup (deps, services, model, init-db)"

deps:
	cargo fetch

check:
	cargo check

build:
	cargo build --release

run:
	cargo run --release

dev:
	cargo watch -x run

services-up:
	docker-compose up -d
	@echo "Waiting for services to be ready..."
	@sleep 5
	@echo "Services are ready!"

services-down:
	docker-compose down

model:
	@echo "Downloading model files from Hugging Face..."
	@mkdir -p models/all-MiniLM-L6-v2-onnx
	@echo "Downloading vocab.txt..."
	@curl -L -o models/all-MiniLM-L6-v2-onnx/vocab.txt \
		https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/vocab.txt
	@echo "Downloading tokenizer.json..."
	@curl -L -o models/all-MiniLM-L6-v2-onnx/tokenizer.json \
		https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json
	@echo "Downloading tokenizer_config.json..."
	@curl -L -o models/all-MiniLM-L6-v2-onnx/tokenizer_config.json \
		https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer_config.json
	@echo "Downloading config.json..."
	@curl -L -o models/all-MiniLM-L6-v2-onnx/config.json \
		https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json
	@echo "Downloading special_tokens_map.json..."
	@curl -L -o models/all-MiniLM-L6-v2-onnx/special_tokens_map.json \
		https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/special_tokens_map.json
	@echo "Downloading ONNX model (this may take a while, ~86MB)..."
	@curl -L -o models/all-MiniLM-L6-v2-onnx/model.onnx \
		https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx
	@echo "✅ Model downloaded successfully!"

init-db:
	./scripts/init_db.sh admin@example.com scale

create-key:
	@echo "Usage: make create-key EMAIL=user@example.com TIER=free"
	@echo "Example: make create-key EMAIL=john@example.com TIER=pro"
	@if [ -z "$(EMAIL)" ]; then \
		echo "Error: EMAIL is required"; \
		echo "Run: make create-key EMAIL=your@email.com TIER=free"; \
		exit 1; \
	fi
	@cargo run --bin create_api_key -- $(EMAIL) $(or $(TIER),free)

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

test:
	cargo test

bench:
	cargo bench

bench-cache:
	cargo bench --bench cache_bench

bench-tokenizer:
	cargo bench --bench tokenizer_bench

bench-inference:
	cargo bench --bench inference_bench

perf-test:
	@echo "Running full performance test suite..."
	@./scripts/performance/run_benchmarks.sh

load-test: load-test-wrk load-test-k6

load-test-k6:
	@echo "Running k6 load tests..."
	@if ! command -v k6 &> /dev/null; then \
		echo "Error: k6 is not installed"; \
		echo "Install with: brew install k6 (macOS)"; \
		echo "Or visit: https://grafana.com/docs/k6/latest/set-up/install-k6/"; \
		exit 1; \
	fi
	@if ! curl -s http://localhost:8000/health > /dev/null 2>&1; then \
		echo "Error: Server is not running on http://localhost:8000"; \
		echo "Start server with: make run"; \
		exit 1; \
	fi
	k6 run scripts/performance/k6_test.js

load-test-wrk:
	@echo "Running wrk load tests..."
	@if ! command -v wrk &> /dev/null; then \
		echo "Error: wrk is not installed"; \
		echo "Install with: brew install wrk (macOS) or apt-get install wrk (Ubuntu)"; \
		exit 1; \
	fi
	@if ! curl -s http://localhost:8000/health > /dev/null 2>&1; then \
		echo "Error: Server is not running on http://localhost:8000"; \
		echo "Start server with: make run"; \
		exit 1; \
	fi
	./scripts/performance/wrk_test.sh

quick-test:
	@if ! command -v k6 &> /dev/null; then \
		echo "Error: k6 is not installed"; \
		echo "Install with: brew install k6 (macOS)"; \
		echo "Or visit: https://grafana.com/docs/k6/latest/set-up/install-k6/"; \
		exit 1; \
	fi
	@if ! curl -s http://localhost:8000/health > /dev/null 2>&1; then \
		echo "Warning: Server might not be running on http://localhost:8000"; \
		echo "Start server with: make run"; \
		echo ""; \
	fi
	@NUM_REQUESTS=$${NUM_REQUESTS:-100} \
		NUM_USERS=$${NUM_USERS:-1} \
		API_KEY=$${API_KEY:-fe_test_key_here} \
		TEXT="$${TEXT:-how to reset password}" \
		API_URL=$${API_URL:-http://localhost:8000/v1/embed} \
		k6 run --quiet --summary-trend-stats="min,avg,med,max,p(90),p(95),p(99)" scripts/performance/quick_test.js

clean:
	cargo clean
	rm -rf target/

logs:
	docker-compose logs -f

docker-build:
	docker-compose -f docker-compose.prod.yml build

docker-up:
	docker-compose -f docker-compose.prod.yml up -d

docker-down:
	docker-compose -f docker-compose.prod.yml down

deploy:
	./scripts/deployment/deploy.sh

quick-deploy:
	./scripts/deployment/quick-deploy.sh

health-check:
	./scripts/deployment/health-check.sh

backup:
	./scripts/deployment/backup.sh

create-api-key:
	./scripts/deployment/create-api-key.sh

logs-prod:
	docker-compose -f docker-compose.prod.yml logs -f

setup: deps services-up model init-db
	@echo ""
	@echo "✅ Setup complete! Run 'make run' to start the server."
