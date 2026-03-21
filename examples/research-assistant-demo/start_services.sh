#!/bin/bash
# Start all services for the Research Assistant demo
#
# Usage: ./start_services.sh [config_file]
#
# Arguments:
#   config_file  Optional path to gateway config (relative to project root)
#                Default: examples/research-assistant-demo/gateway-configs/config.yaml
#
# Examples:
#   ./start_services.sh                                    # Use default config
#   ./start_services.sh gateway-configs/minimal_config.yaml  # Use minimal config (relative to demo dir)
#
# This script starts:
# - 5 MCP backend services (search, fetch, entity, category, tag)
# - The AgentGateway
# - The research agent
#
# Uses tmux for managing multiple services in one terminal.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse config file argument
if [ -n "$1" ]; then
    # If argument is provided, check if it's relative to demo dir or project root
    if [ -f "$SCRIPT_DIR/$1" ]; then
        GATEWAY_CONFIG="examples/research-assistant-demo/$1"
    elif [ -f "$PROJECT_ROOT/$1" ]; then
        GATEWAY_CONFIG="$1"
    else
        echo -e "${RED}Error: Config file not found: $1${NC}"
        echo "Looked in:"
        echo "  - $SCRIPT_DIR/$1"
        echo "  - $PROJECT_ROOT/$1"
        exit 1
    fi
else
    GATEWAY_CONFIG="examples/research-assistant-demo/gateway-configs/config.yaml"
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Research Assistant Demo - Starting Services${NC}"
echo -e "${GREEN}========================================${NC}"

# Check for tmux
if ! command -v tmux &> /dev/null; then
    echo -e "${RED}Error: tmux is required but not installed.${NC}"
    echo "Install with: apt install tmux (Linux) or brew install tmux (macOS)"
    exit 1
fi

# Check for uv
if ! command -v uv &> /dev/null; then
    echo -e "${RED}Error: uv is required but not installed.${NC}"
    echo "Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

cd "$SCRIPT_DIR"

# Load environment variables from .env if it exists
if [ -f ".env" ]; then
    echo -e "\n${YELLOW}Loading environment from .env...${NC}"
    set -a  # automatically export all variables
    source .env
    set +a
else
    echo -e "\n${YELLOW}Warning: No .env file found. API keys may not be set.${NC}"
    echo "Copy .env.example to .env and add your API keys."
fi

# Install dependencies
echo -e "\n${YELLOW}Installing Python dependencies...${NC}"
uv sync --quiet

# Seed databases if they don't exist
if [ ! -f "data/entities.db" ] || [ ! -f "data/categories.db" ] || [ ! -f "data/tags.db" ]; then
    echo -e "\n${YELLOW}Seeding databases...${NC}"
    uv run python data/seed_data.py
fi

# Kill existing session if it exists
tmux kill-session -t research-demo 2>/dev/null || true

# Create new tmux session
echo -e "\n${YELLOW}Starting services in tmux session 'research-demo'...${NC}"
tmux new-session -d -s research-demo -n services

# Create logs directory
mkdir -p "$SCRIPT_DIR/logs"

# Start Search Service (8001)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Search Service on :8001...' && uv run python -m mcp_tools.search_service --port 8001 2>&1 | tee logs/search.log" C-m
sleep 1
tmux split-window -h -t research-demo

# Start Fetch Service (8002)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Fetch Service on :8002...' && uv run python -m mcp_tools.fetch_service --port 8002 2>&1 | tee logs/fetch.log" C-m
sleep 1
tmux split-window -v -t research-demo

# Start Entity Service (8003)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Entity Service on :8003...' && uv run python -m mcp_tools.entity_service --port 8003 2>&1 | tee logs/entity.log" C-m
sleep 1

# Select first pane and split
tmux select-pane -t research-demo:0.0
tmux split-window -v -t research-demo

# Start Category Service (8004)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Category Service on :8004...' && uv run python -m mcp_tools.category_service --port 8004 2>&1 | tee logs/category.log" C-m
sleep 1
tmux split-window -v -t research-demo

# Start Tag Service (8005)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Tag Service on :8005...' && uv run python -m mcp_tools.tag_service --port 8005 2>&1 | tee logs/tag.log" C-m
sleep 1

# Create new window for gateway
tmux new-window -t research-demo -n gateway
tmux send-keys -t research-demo "cd '$PROJECT_ROOT' && echo 'Starting AgentGateway on :3000 with $GATEWAY_CONFIG...' && sleep 3 && ./target/debug/agentgateway -f $GATEWAY_CONFIG 2>&1 | tee '$SCRIPT_DIR/logs/gateway.log'" C-m

# Create new window for agent (using ADK's get_fast_api_app pattern)
tmux new-window -t research-demo -n agent
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Research Agent on :9001...' && sleep 5 && GATEWAY_URL=http://localhost:3000 uv run python main.py 2>&1 | tee logs/agent.log" C-m

# Create new window for web UI
tmux new-window -t research-demo -n webui
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Web UI on :8080...' && sleep 7 && uv run python -m web_ui.chat_app 2>&1 | tee logs/webui.log" C-m

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Services Starting!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "Gateway config: ${YELLOW}$GATEWAY_CONFIG${NC}"
echo ""
echo "Services:"
echo "  - Search Service:   http://localhost:8001/mcp"
echo "  - Fetch Service:    http://localhost:8002/mcp"
echo "  - Entity Service:   http://localhost:8003/mcp"
echo "  - Category Service: http://localhost:8004/mcp"
echo "  - Tag Service:      http://localhost:8005/mcp"
echo "  - Gateway:          http://localhost:3000/mcp"
echo "  - Research Agent:   http://localhost:9001"
echo "  - Web UI:           http://localhost:8080"
echo ""
echo -e "To view logs: ${YELLOW}tmux attach -t research-demo${NC}"
echo -e "To stop:      ${YELLOW}./stop_services.sh${NC} or ${YELLOW}tmux kill-session -t research-demo${NC}"
echo ""
echo -e "${GREEN}Open the Web UI:${NC}"
echo "  http://localhost:8080"
echo ""
echo -e "${GREEN}Or test via curl:${NC}"
echo '  curl -X POST http://localhost:9001/chat \'
echo '    -H "Content-Type: application/json" \'
echo '    -d '\''{"message":"Research transformer alternatives for 2025-2026"}'\'''
