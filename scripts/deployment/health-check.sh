#!/bin/bash
set -euo pipefail

# Smally Health Check Script
# Verifies all services are healthy

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

SUCCESS=0
FAILED=0

log_success() {
    echo -e "${GREEN}✓${NC} $1"
    SUCCESS=$((SUCCESS+1))
}

log_failure() {
    echo -e "${RED}✗${NC} $1"
    FAILED=$((FAILED+1))
}

log_info() {
    echo -e "${YELLOW}→${NC} $1"
}

cd "$PROJECT_ROOT"

echo "Smally Health Check"
echo "======================"
echo ""

# Check if Docker is running
log_info "Checking Docker..."
if docker info >/dev/null 2>&1; then
    log_success "Docker is running"
else
    log_failure "Docker is not running"
    exit 1
fi

# Check if services are running
log_info "Checking services..."
SERVICES=("postgres" "redis" "app" "nginx" "prometheus" "grafana")

for service in "${SERVICES[@]}"; do
    if docker-compose -f docker-compose.prod.yml ps | grep -q "$service.*Up"; then
        log_success "$service is running"
    else
        log_failure "$service is not running"
    fi
done

# Check service health
log_info "Checking service health..."

# PostgreSQL
if docker-compose -f docker-compose.prod.yml exec -T postgres pg_isready -U smally >/dev/null 2>&1; then
    log_success "PostgreSQL is healthy"
else
    log_failure "PostgreSQL is unhealthy"
fi

# Redis
if docker-compose -f docker-compose.prod.yml exec -T redis redis-cli ping | grep -q "PONG"; then
    log_success "Redis is healthy"
else
    log_failure "Redis is unhealthy"
fi

# API Health endpoint
if curl -sf http://localhost:8000/health >/dev/null 2>&1; then
    log_success "API health endpoint responding"
else
    log_failure "API health endpoint not responding"
fi

# Nginx
if curl -sf http://localhost:80 >/dev/null 2>&1 || curl -sf https://localhost:443 >/dev/null 2>&1; then
    log_success "Nginx is responding"
else
    log_failure "Nginx is not responding"
fi

# Prometheus
if curl -sf http://localhost:9090/-/healthy >/dev/null 2>&1; then
    log_success "Prometheus is healthy"
else
    log_failure "Prometheus is unhealthy"
fi

# Grafana
if curl -sf http://localhost:3000/api/health >/dev/null 2>&1; then
    log_success "Grafana is healthy"
else
    log_failure "Grafana is unhealthy"
fi

# Check disk space
log_info "Checking disk space..."
DISK_USAGE=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -lt 80 ]; then
    log_success "Disk usage is OK ($DISK_USAGE%)"
else
    log_failure "Disk usage is high ($DISK_USAGE%)"
fi

# Check memory
log_info "Checking memory..."
MEM_USAGE=$(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100}')
if [ "$MEM_USAGE" -lt 90 ]; then
    log_success "Memory usage is OK ($MEM_USAGE%)"
else
    log_failure "Memory usage is high ($MEM_USAGE%)"
fi

# Summary
echo ""
echo "======================"
echo "Summary: $SUCCESS passed, $FAILED failed"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All checks passed!${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed!${NC}"
    exit 1
fi
