#!/usr/bin/env bash
#
# start_gateway.sh - Launch agentgateway with pattern demo configurations
#
# Usage:
#   ./start_gateway.sh              # Start with default config
#   ./start_gateway.sh --build      # Build first, then start
#   ./start_gateway.sh --docker     # Run via docker-compose
#   ./start_gateway.sh --config X   # Use specific config file
#
# Environment variables:
#   GATEWAY_PORT     - Main MCP/HTTP port (default: 3000)
#   ADMIN_PORT       - Admin UI port (default: 15000)
#   LOG_LEVEL        - Log level (default: info)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Defaults
GATEWAY_PORT="${GATEWAY_PORT:-3000}"
ADMIN_PORT="${ADMIN_PORT:-15000}"
LOG_LEVEL="${LOG_LEVEL:-info}"
CONFIG_FILE="$SCRIPT_DIR/configs/demo-config.yaml"
BUILD=false
USE_DOCKER=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║            AgentGateway Pattern Demos                     ║"
    echo "╠═══════════════════════════════════════════════════════════╣"
    echo "║  MCP Endpoint:  http://localhost:$GATEWAY_PORT/mcp                  ║"
    echo "║  SSE Endpoint:  http://localhost:$GATEWAY_PORT/sse                  ║"
    echo "║  A2A Endpoint:  http://localhost:$GATEWAY_PORT/a2a                  ║"
    echo "║  Admin UI:      http://localhost:$ADMIN_PORT/ui                   ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Start agentgateway with pattern demo configurations.

Options:
  -b, --build       Build agentgateway before starting
  -d, --docker      Run via docker-compose
  -c, --config FILE Use specified config file
  -h, --help        Show this help message

Environment Variables:
  GATEWAY_PORT      Main MCP/HTTP port (default: 3000)
  ADMIN_PORT        Admin UI port (default: 15000)
  LOG_LEVEL         Log level: debug, info, warn, error (default: info)

Examples:
  $(basename "$0")                    # Start with default config
  $(basename "$0") --build            # Build and start
  $(basename "$0") --docker           # Run in Docker
  LOG_LEVEL=debug $(basename "$0")    # Start with debug logging
EOF
}

check_dependencies() {
    local missing=()

    if ! command -v npx &> /dev/null; then
        missing+=("npx (Node.js)")
    fi

    if ! command -v uvx &> /dev/null; then
        missing+=("uvx (uv package manager)")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        echo -e "${YELLOW}Warning: Some MCP servers may not work without:${NC}"
        for dep in "${missing[@]}"; do
            echo "  - $dep"
        done
        echo ""
    fi
}

build_gateway() {
    echo -e "${BLUE}Building agentgateway...${NC}"
    cd "$PROJECT_ROOT"

    # Build UI if needed
    if [[ ! -d "ui/out" ]]; then
        echo "Building UI..."
        (cd ui && npm install && npm run build)
    fi

    # Build gateway
    make build
    echo -e "${GREEN}Build complete!${NC}"
}

start_docker() {
    echo -e "${BLUE}Starting with Docker Compose...${NC}"
    cd "$SCRIPT_DIR"
    docker-compose up --build
}

start_native() {
    local binary="$PROJECT_ROOT/target/release/agentgateway"

    # Check if binary exists
    if [[ ! -x "$binary" ]]; then
        echo -e "${YELLOW}agentgateway binary not found. Building...${NC}"
        build_gateway
    fi

    print_banner
    check_dependencies

    echo -e "${GREEN}Starting agentgateway...${NC}"
    echo "Config: $CONFIG_FILE"
    echo "Log level: $LOG_LEVEL"
    echo ""

    cd "$PROJECT_ROOT"
    RUST_LOG="$LOG_LEVEL" exec "$binary" -f "$CONFIG_FILE"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -b|--build)
            BUILD=true
            shift
            ;;
        -d|--docker)
            USE_DOCKER=true
            shift
            ;;
        -c|--config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Main execution
if [[ "$BUILD" == "true" ]]; then
    build_gateway
fi

if [[ "$USE_DOCKER" == "true" ]]; then
    start_docker
else
    start_native
fi
