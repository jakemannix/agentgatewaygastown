#!/bin/bash
#
# Pattern Demos Gateway Startup Script
#
# This script starts agentgateway with the pattern demos configuration,
# showcasing all composition patterns available in agentgateway.
#
# Usage:
#   ./start.sh              # Start with default settings
#   ./start.sh --dev        # Start in development mode with hot-reload
#   ./start.sh --port 8080  # Start on a custom port
#
# Prerequisites:
#   - agentgateway binary in PATH or AGENTGATEWAY_BIN set
#   - Mock MCP servers (or use --mock flag for built-in mocks)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/config.yaml"
REGISTRY_FILE="$SCRIPT_DIR/patterns_registry.json"

# Default settings
PORT=3000
LOG_LEVEL="${LOG_LEVEL:-info}"
DEV_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dev)
            DEV_MODE=true
            LOG_LEVEL="debug"
            shift
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        -h|--help)
            echo "Pattern Demos Gateway - Agentgateway Composition Patterns Showcase"
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --dev              Enable development mode with debug logging"
            echo "  --port PORT        Set the gateway port (default: 3000)"
            echo "  --log-level LEVEL  Set log level (trace|debug|info|warn|error)"
            echo "  -h, --help         Show this help message"
            echo ""
            echo "Patterns demonstrated:"
            echo "  - Pipeline: search_and_summarize"
            echo "  - Saga: create_project (with compensation)"
            echo "  - ScatterGather: multi_search"
            echo "  - Cache: cached_user_lookup"
            echo "  - Retry: reliable_notification"
            echo "  - CircuitBreaker: protected_external_call"
            echo "  - Timeout: time_bounded_search"
            echo ""
            echo "Environment variables:"
            echo "  AGENTGATEWAY_BIN   Path to agentgateway binary"
            echo "  LOG_LEVEL          Default log level"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Find agentgateway binary
if [[ -n "$AGENTGATEWAY_BIN" ]]; then
    AGENTGATEWAY="$AGENTGATEWAY_BIN"
elif command -v agentgateway &> /dev/null; then
    AGENTGATEWAY="agentgateway"
elif [[ -f "$SCRIPT_DIR/../../../target/release/agentgateway" ]]; then
    AGENTGATEWAY="$SCRIPT_DIR/../../../target/release/agentgateway"
elif [[ -f "$SCRIPT_DIR/../../../target/debug/agentgateway" ]]; then
    AGENTGATEWAY="$SCRIPT_DIR/../../../target/debug/agentgateway"
else
    echo "Error: agentgateway binary not found"
    echo "Please either:"
    echo "  - Add agentgateway to your PATH"
    echo "  - Set AGENTGATEWAY_BIN environment variable"
    echo "  - Build with: cargo build --release"
    exit 1
fi

# Verify config files exist
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Error: Config file not found: $CONFIG_FILE"
    exit 1
fi

if [[ ! -f "$REGISTRY_FILE" ]]; then
    echo "Error: Registry file not found: $REGISTRY_FILE"
    exit 1
fi

# Print startup info
echo "=========================================="
echo "  Pattern Demos Gateway"
echo "=========================================="
echo ""
echo "Configuration: $CONFIG_FILE"
echo "Registry:      $REGISTRY_FILE"
echo "Port:          $PORT"
echo "Log Level:     $LOG_LEVEL"
echo "Dev Mode:      $DEV_MODE"
echo ""
echo "Patterns available:"
echo "  - search_and_summarize (Pipeline)"
echo "  - create_project (Saga)"
echo "  - multi_search (ScatterGather)"
echo "  - cached_user_lookup (Cache)"
echo "  - reliable_notification (Retry)"
echo "  - protected_external_call (CircuitBreaker)"
echo "  - time_bounded_search (Timeout)"
echo ""
echo "Starting gateway on http://localhost:$PORT"
echo "=========================================="
echo ""

# Build environment
export RUST_LOG="${RUST_LOG:-agentgateway=$LOG_LEVEL}"

# Modify config port if custom
if [[ "$PORT" != "3000" ]]; then
    # Create temp config with modified port
    TEMP_CONFIG=$(mktemp)
    sed "s/port: 3000/port: $PORT/" "$CONFIG_FILE" > "$TEMP_CONFIG"
    CONFIG_FILE="$TEMP_CONFIG"
    trap "rm -f $TEMP_CONFIG" EXIT
fi

# Start agentgateway
exec "$AGENTGATEWAY" --config "$CONFIG_FILE"
