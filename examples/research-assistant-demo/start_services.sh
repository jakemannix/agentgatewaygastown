#!/bin/bash
# Start all services for the Research Assistant demo
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

# Start Search Service (8001)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Search Service on :8001...' && uv run python -m mcp_tools.search_service --port 8001" C-m
sleep 1
tmux split-window -h -t research-demo

# Start Fetch Service (8002)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Fetch Service on :8002...' && uv run python -m mcp_tools.fetch_service --port 8002" C-m
sleep 1
tmux split-window -v -t research-demo

# Start Entity Service (8003)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Entity Service on :8003...' && uv run python -m mcp_tools.entity_service --port 8003" C-m
sleep 1

# Select first pane and split
tmux select-pane -t research-demo:0.0
tmux split-window -v -t research-demo

# Start Category Service (8004)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Category Service on :8004...' && uv run python -m mcp_tools.category_service --port 8004" C-m
sleep 1
tmux split-window -v -t research-demo

# Start Tag Service (8005)
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Tag Service on :8005...' && uv run python -m mcp_tools.tag_service --port 8005" C-m
sleep 1

# Create new window for gateway
tmux new-window -t research-demo -n gateway
tmux send-keys -t research-demo "cd '$PROJECT_ROOT' && echo 'Starting AgentGateway on :3000...' && sleep 3 && ./target/release/agentgateway -f examples/research-assistant-demo/gateway-configs/config.yaml" C-m

# Create new window for agent
tmux new-window -t research-demo -n agent
tmux send-keys -t research-demo "cd '$SCRIPT_DIR' && echo 'Starting Research Agent on :9001...' && sleep 5 && uv run python -m agents.research_agent --port 9001 --gateway-url http://localhost:3000/mcp" C-m

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Services Starting!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Services:"
echo "  - Search Service:   http://localhost:8001/mcp"
echo "  - Fetch Service:    http://localhost:8002/mcp"
echo "  - Entity Service:   http://localhost:8003/mcp"
echo "  - Category Service: http://localhost:8004/mcp"
echo "  - Tag Service:      http://localhost:8005/mcp"
echo "  - Gateway:          http://localhost:3000/mcp"
echo "  - Research Agent:   http://localhost:9001"
echo ""
echo -e "To view logs: ${YELLOW}tmux attach -t research-demo${NC}"
echo -e "To stop:      ${YELLOW}./stop_services.sh${NC} or ${YELLOW}tmux kill-session -t research-demo${NC}"
echo ""
echo -e "${GREEN}Test the agent:${NC}"
echo '  curl -X POST http://localhost:9001/chat \'
echo '    -H "Content-Type: application/json" \'
echo '    -d '\''{"message":"Research transformer alternatives for 2025-2026"}'\'''
