#!/bin/bash
# Start all eCommerce demo services
#
# This script starts all services needed for the demo:
# - MCP Backend Services (HTTP on ports 8001-8005)
# - AgentGateway (MCP proxy with virtual tools on port 3000)
# - Customer Agent (Google ADK, port 9001)
# - Merchandiser Agent (LangGraph, port 9002)
# - Chat Web UI (FastHTML on port 8080)
#
# Usage:
#   ./start_services.sh         # Start all services
#   ./start_services.sh --seed  # Seed databases first, then start

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔══════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     eCommerce Demo Startup           ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════╝${NC}"
echo ""

# Require uv
if ! command -v uv &> /dev/null; then
    echo -e "${RED}Error: uv is required but not installed.${NC}"
    echo "Install uv: https://docs.astral.sh/uv/getting-started/installation/"
    echo "  curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

# Install/update dependencies with uv
if [ ! -d ".venv" ] || [ "pyproject.toml" -nt ".venv" ]; then
    echo -e "${YELLOW}Installing/updating Python dependencies with uv...${NC}"
    uv sync
    echo -e "${GREEN}Dependencies installed!${NC}"
    echo ""
fi

# Check if --seed flag is passed
if [ "$1" == "--seed" ]; then
    echo -e "${YELLOW}Seeding databases...${NC}"
    uv run python data/seed_data.py
    echo -e "${GREEN}Databases seeded!${NC}"
    echo ""
fi

# Check if databases exist, seed if not
if [ ! -f "data/catalog.db" ]; then
    echo -e "${YELLOW}Databases not found. Running seed script...${NC}"
    uv run python data/seed_data.py
    echo ""
fi

# Check for required API key
if [ -z "$ANTHROPIC_API_KEY" ] && [ -z "$OPENAI_API_KEY" ] && [ -z "$GOOGLE_API_KEY" ]; then
    echo -e "${YELLOW}Warning: No LLM API key found.${NC}"
    echo -e "Set one of: ANTHROPIC_API_KEY, OPENAI_API_KEY, or GOOGLE_API_KEY"
    echo -e "Agents will fall back to direct tool calls without LLM reasoning."
    echo ""
fi

# Check for tmux
if command -v tmux &> /dev/null; then
    echo -e "${GREEN}Starting services with tmux...${NC}"

    # Kill existing session if it exists
    tmux kill-session -t ecommerce-demo 2>/dev/null || true

    # Create new tmux session with MCP services window
    tmux new-session -d -s ecommerce-demo -n mcp-services

    # Start all 5 MCP services in the first window (split panes)
    # Catalog Service (port 8001)
    tmux send-keys -t ecommerce-demo:mcp-services "cd $SCRIPT_DIR && uv run python -m mcp_tools.catalog_service.server" C-m
    tmux split-window -t ecommerce-demo:mcp-services -h

    # Cart Service (port 8002)
    tmux send-keys -t ecommerce-demo:mcp-services "cd $SCRIPT_DIR && uv run python -m mcp_tools.cart_service.server" C-m
    tmux split-window -t ecommerce-demo:mcp-services -v

    # Order Service (port 8003)
    tmux send-keys -t ecommerce-demo:mcp-services "cd $SCRIPT_DIR && uv run python -m mcp_tools.order_service.server" C-m

    # Select pane 0 and split for more services
    tmux select-pane -t ecommerce-demo:mcp-services.0
    tmux split-window -t ecommerce-demo:mcp-services -v

    # Inventory Service (port 8004)
    tmux send-keys -t ecommerce-demo:mcp-services "cd $SCRIPT_DIR && uv run python -m mcp_tools.inventory_service.server" C-m

    # Select pane 2 and split
    tmux select-pane -t ecommerce-demo:mcp-services.2
    tmux split-window -t ecommerce-demo:mcp-services -v

    # Supplier Service (port 8005)
    tmux send-keys -t ecommerce-demo:mcp-services "cd $SCRIPT_DIR && uv run python -m mcp_tools.supplier_service.server" C-m

    # Wait for MCP services to start
    echo -e "${BLUE}Waiting for MCP services to start...${NC}"
    sleep 3

    # Create window for gateway
    tmux new-window -t ecommerce-demo -n gateway
    tmux send-keys -t ecommerce-demo:gateway "cd $SCRIPT_DIR/../.. && ./target/release/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml" C-m

    # Wait for gateway to start
    sleep 2

    # Create window for customer agent
    tmux new-window -t ecommerce-demo -n customer-agent
    tmux send-keys -t ecommerce-demo:customer-agent "cd $SCRIPT_DIR && uv run python -m agents.customer_agent" C-m

    # Create window for merchandiser agent
    tmux new-window -t ecommerce-demo -n merch-agent
    tmux send-keys -t ecommerce-demo:merch-agent "cd $SCRIPT_DIR && uv run python -m agents.merchandiser_agent" C-m

    # Wait for agents to start
    sleep 2

    # Create window for chat UI
    tmux new-window -t ecommerce-demo -n chat-ui
    tmux send-keys -t ecommerce-demo:chat-ui "cd $SCRIPT_DIR && uv run python web_ui/chat_app.py" C-m

    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║        All services started!         ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BLUE}MCP Backend Services (HTTP):${NC}"
    echo "  - Catalog Service:   http://localhost:8001"
    echo "  - Cart Service:      http://localhost:8002"
    echo "  - Order Service:     http://localhost:8003"
    echo "  - Inventory Service: http://localhost:8004"
    echo "  - Supplier Service:  http://localhost:8005"
    echo ""
    echo -e "${BLUE}Gateway & Agents:${NC}"
    echo "  - Gateway (MCP):      http://localhost:3000"
    echo "  - Gateway UI:         http://localhost:15000/ui"
    echo "  - Customer Agent:     http://localhost:9001 (REST /chat + A2A)"
    echo "  - Merchandiser Agent: http://localhost:9002 (REST /chat + A2A)"
    echo ""
    echo -e "${BLUE}Web UI:${NC}"
    echo "  - Chat Web UI:        http://localhost:8080"
    echo ""
    echo -e "${BLUE}tmux controls:${NC}"
    echo "  Attach:  tmux attach -t ecommerce-demo"
    echo "  Windows: Ctrl+B then 0-4 (mcp-services, gateway, customer-agent, merch-agent, chat-ui)"
    echo "  Kill:    tmux kill-session -t ecommerce-demo"
    echo ""

    # Attach to session
    tmux attach -t ecommerce-demo
else
    echo -e "${YELLOW}tmux not found. Starting services manually...${NC}"
    echo ""
    echo "Please run these commands in separate terminals:"
    echo ""
    echo -e "${BLUE}Terminal 1-5 (MCP Services):${NC}"
    echo "  cd $SCRIPT_DIR && uv run python -m mcp_tools.catalog_service.server    # Port 8001"
    echo "  cd $SCRIPT_DIR && uv run python -m mcp_tools.cart_service.server       # Port 8002"
    echo "  cd $SCRIPT_DIR && uv run python -m mcp_tools.order_service.server      # Port 8003"
    echo "  cd $SCRIPT_DIR && uv run python -m mcp_tools.inventory_service.server  # Port 8004"
    echo "  cd $SCRIPT_DIR && uv run python -m mcp_tools.supplier_service.server   # Port 8005"
    echo ""
    echo -e "${BLUE}Terminal 6 (Gateway):${NC}"
    echo "  cd $(dirname $SCRIPT_DIR)/.. && ./target/release/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml"
    echo ""
    echo -e "${BLUE}Terminal 7 (Customer Agent):${NC}"
    echo "  cd $SCRIPT_DIR && uv run python -m agents.customer_agent"
    echo ""
    echo -e "${BLUE}Terminal 8 (Merchandiser Agent):${NC}"
    echo "  cd $SCRIPT_DIR && uv run python -m agents.merchandiser_agent"
    echo ""
    echo -e "${BLUE}Terminal 9 (Chat UI):${NC}"
    echo "  cd $SCRIPT_DIR && uv run python web_ui/chat_app.py"
    echo ""
fi
