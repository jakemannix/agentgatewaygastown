#!/bin/bash
# Start custom MCP services for pattern demos
#
# These services complement the pre-built MCP servers (fetch, memory, time)
# with custom functionality backed by SQLite databases.
#
# Usage:
#   ./start_mcp_services.sh           # Start all services
#   ./start_mcp_services.sh --check   # Check if services are running
#
# Services are started in the background with logs in /tmp/mcp-*.log

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_TOOLS_DIR="$SCRIPT_DIR/mcp-tools"
PIDS_DIR="/tmp/mcp-service-pids"
LOGS_DIR="/tmp"

# Service configuration (port assignments)
DOCUMENT_SERVICE_PORT=8001
TASK_SERVICE_PORT=8002
USER_SERVICE_PORT=8003
NOTIFICATION_SERVICE_PORT=8004

# Database paths - configurable via environment variable
# Default: ./demo_data (relative to pattern-demos directory, persists across reboots)
# Override: export MCP_DATA_DIR=/path/to/persistent/storage
#
# Directory structure:
#   demo_data/
#   ├── document-service/
#   │   └── documents.db
#   ├── task-service/
#   │   └── tasks.db
#   ├── user-service/
#   │   └── users.db
#   └── notification-service/
#       └── notifications.db
#
DATA_BASE_DIR="${MCP_DATA_DIR:-$SCRIPT_DIR/demo_data}"
mkdir -p "$DATA_BASE_DIR/document-service"
mkdir -p "$DATA_BASE_DIR/task-service"
mkdir -p "$DATA_BASE_DIR/user-service"
mkdir -p "$DATA_BASE_DIR/notification-service"
mkdir -p "$PIDS_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_port() {
    local port=$1
    if lsof -i :$port >/dev/null 2>&1; then
        return 0  # Port in use
    else
        return 1  # Port free
    fi
}

wait_for_port() {
    local port=$1
    local service=$2
    local max_wait=30
    local waited=0
    
    while ! check_port $port; do
        sleep 1
        waited=$((waited + 1))
        if [ $waited -ge $max_wait ]; then
            log_error "$service failed to start on port $port after ${max_wait}s"
            return 1
        fi
    done
    return 0
}

start_document_service() {
    local port=$DOCUMENT_SERVICE_PORT
    local db_path="$DATA_BASE_DIR/document-service/documents.db"
    local log_file="$LOGS_DIR/mcp-document-service.log"
    local pid_file="$PIDS_DIR/document-service.pid"
    
    if check_port $port; then
        log_warn "document-service port $port already in use, skipping"
        return 0
    fi
    
    log_info "Starting document-service on port $port..."
    log_info "  Database: $db_path"
    
    cd "$MCP_TOOLS_DIR/document_service"
    uv run python -m document_service \
        --transport streamable-http \
        --port $port \
        --db-path "$db_path" \
        > "$log_file" 2>&1 &
    
    echo $! > "$pid_file"
    
    if wait_for_port $port "document-service"; then
        log_success "document-service running on port $port (PID: $(cat $pid_file))"
    fi
}

start_task_service() {
    local port=$TASK_SERVICE_PORT
    local db_path="$DATA_BASE_DIR/task-service/tasks.db"
    local log_file="$LOGS_DIR/mcp-task-service.log"
    local pid_file="$PIDS_DIR/task-service.pid"
    
    if check_port $port; then
        log_warn "task-service port $port already in use, skipping"
        return 0
    fi
    
    log_info "Starting task-service on port $port..."
    log_info "  Database: $db_path"
    
    cd "$MCP_TOOLS_DIR/task_service"
    uv run python -m task_service \
        --transport streamable-http \
        --port $port \
        --db "$db_path" \
        > "$log_file" 2>&1 &
    
    echo $! > "$pid_file"
    
    if wait_for_port $port "task-service"; then
        log_success "task-service running on port $port (PID: $(cat $pid_file))"
    fi
}

start_user_service() {
    local port=$USER_SERVICE_PORT
    local db_path="$DATA_BASE_DIR/user-service/users.db"
    local log_file="$LOGS_DIR/mcp-user-service.log"
    local pid_file="$PIDS_DIR/user-service.pid"
    
    if check_port $port; then
        log_warn "user-service port $port already in use, skipping"
        return 0
    fi
    
    log_info "Starting user-service on port $port..."
    log_info "  Database: $db_path"
    
    cd "$MCP_TOOLS_DIR/user_service"
    uv run python server.py \
        --transport streamable-http \
        --port $port \
        --db "$db_path" \
        --seed \
        > "$log_file" 2>&1 &
    
    echo $! > "$pid_file"
    
    if wait_for_port $port "user-service"; then
        log_success "user-service running on port $port (PID: $(cat $pid_file))"
    fi
}

start_notification_service() {
    local port=$NOTIFICATION_SERVICE_PORT
    local db_path="$DATA_BASE_DIR/notification-service/notifications.db"
    local log_file="$LOGS_DIR/mcp-notification-service.log"
    local pid_file="$PIDS_DIR/notification-service.pid"
    
    if check_port $port; then
        log_warn "notification-service port $port already in use, skipping"
        return 0
    fi
    
    log_info "Starting notification-service on port $port..."
    log_info "  Database: $db_path"
    
    cd "$MCP_TOOLS_DIR/notification_service"
    uv run python server.py \
        --transport streamable-http \
        --port $port \
        --db "$db_path" \
        > "$log_file" 2>&1 &
    
    echo $! > "$pid_file"
    
    if wait_for_port $port "notification-service"; then
        log_success "notification-service running on port $port (PID: $(cat $pid_file))"
    fi
}

check_services() {
    echo ""
    echo "MCP Service Status"
    echo "=================="
    
    local all_running=true
    
    for service in document task user notification; do
        local port_var="${service^^}_SERVICE_PORT"
        local port=${!port_var}
        local pid_file="$PIDS_DIR/${service}-service.pid"
        
        if check_port $port; then
            if [ -f "$pid_file" ]; then
                echo -e "  ${service}-service: ${GREEN}running${NC} on port $port (PID: $(cat $pid_file))"
            else
                echo -e "  ${service}-service: ${YELLOW}running${NC} on port $port (external process)"
            fi
        else
            echo -e "  ${service}-service: ${RED}stopped${NC} (port $port)"
            all_running=false
        fi
    done
    
    echo ""
    echo "Log files in: $LOGS_DIR/mcp-*.log"
    echo ""
    
    if $all_running; then
        return 0
    else
        return 1
    fi
}

# Main
case "${1:-}" in
    --check)
        check_services
        ;;
    *)
        echo ""
        echo "╔═══════════════════════════════════════════════════════════╗"
        echo "║           Starting Custom MCP Services                     ║"
        echo "╠═══════════════════════════════════════════════════════════╣"
        echo "║  document-service  : http://localhost:$DOCUMENT_SERVICE_PORT              ║"
        echo "║  task-service      : http://localhost:$TASK_SERVICE_PORT              ║"
        echo "║  user-service      : http://localhost:$USER_SERVICE_PORT              ║"
        echo "║  notification-svc  : http://localhost:$NOTIFICATION_SERVICE_PORT              ║"
        echo "╠═══════════════════════════════════════════════════════════╣"
        echo "║  Data directory: $DATA_BASE_DIR"
        echo "╚═══════════════════════════════════════════════════════════╝"
        echo ""
        
        start_document_service
        start_task_service
        start_user_service
        start_notification_service
        
        echo ""
        check_services
        ;;
esac
