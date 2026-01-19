#!/bin/bash
# Stop custom MCP services
#
# Usage:
#   ./stop_mcp_services.sh           # Stop all services
#   ./stop_mcp_services.sh --force   # Force kill if graceful stop fails

set -e

PIDS_DIR="/tmp/mcp-service-pids"
FORCE="${1:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "[INFO] $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

stop_service() {
    local name=$1
    local pid_file="$PIDS_DIR/${name}.pid"
    
    if [ ! -f "$pid_file" ]; then
        log_warn "$name: no PID file found"
        return 0
    fi
    
    local pid=$(cat "$pid_file")
    
    if ! kill -0 "$pid" 2>/dev/null; then
        log_warn "$name: process $pid not running"
        rm -f "$pid_file"
        return 0
    fi
    
    log_info "Stopping $name (PID: $pid)..."
    
    # Try graceful shutdown first
    kill -TERM "$pid" 2>/dev/null || true
    
    # Wait for process to exit
    local waited=0
    while kill -0 "$pid" 2>/dev/null && [ $waited -lt 5 ]; do
        sleep 1
        waited=$((waited + 1))
    done
    
    # Force kill if still running
    if kill -0 "$pid" 2>/dev/null; then
        if [ "$FORCE" = "--force" ]; then
            log_warn "$name: force killing..."
            kill -9 "$pid" 2>/dev/null || true
        else
            log_warn "$name: still running, use --force to kill"
            return 1
        fi
    fi
    
    rm -f "$pid_file"
    log_success "$name stopped"
}

echo ""
echo "Stopping MCP Services"
echo "====================="
echo ""

for service in document-service task-service user-service notification-service; do
    stop_service "$service"
done

echo ""
echo "All services stopped."
echo ""

# Also kill by port as fallback
for port in 8001 8002 8003 8004; do
    pid=$(lsof -t -i :$port 2>/dev/null || true)
    if [ -n "$pid" ]; then
        log_warn "Killing orphan process on port $port (PID: $pid)"
        kill -9 $pid 2>/dev/null || true
    fi
done
