# AgentGateway Pattern Demos Makefile
#
# Orchestrates the demo environment with targets for different agent frameworks.
#
# Quick start:
#   make setup          # Install dependencies
#   make start-services # Start gateway + MCP servers
#   make demo           # Run interactive demo
#
# Framework-specific demos:
#   make run-claude     # Demo with Claude Code
#   make run-langgraph  # Demo with LangGraph
#   make run-adk        # Demo with Google ADK
#   make demo-a2a       # Demo A2A protocol

.PHONY: help setup start-services stop-services demo \
        run-claude run-langgraph run-adk demo-a2a \
        build clean docker-up docker-down

# Configuration
GATEWAY_PORT ?= 3000
ADMIN_PORT ?= 15000
A2A_PORT ?= 9999
PROJECT_ROOT := $(shell cd ../.. && pwd)

# Colors
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m

help: ## Show this help message
	@echo "AgentGateway Pattern Demos"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Setup & Services:"
	@grep -E '^(setup|start-services|stop-services|build|clean):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'
	@echo ""
	@echo "Demos:"
	@grep -E '^(demo|run-claude|run-langgraph|run-adk|demo-a2a):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'
	@echo ""
	@echo "Docker:"
	@grep -E '^(docker-up|docker-down):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'

# =============================================================================
# Setup & Build
# =============================================================================

setup: ## Install all dependencies (Python, Node, Rust build)
	@echo "$(BLUE)Setting up dependencies...$(NC)"
	@# Python dependencies for demo script
	pip install httpx rich 2>/dev/null || pip3 install httpx rich
	@# Check for Node.js (needed for MCP servers)
	@command -v npx >/dev/null 2>&1 || { echo "$(YELLOW)Warning: npx not found. Install Node.js for MCP servers.$(NC)"; }
	@# Check for uv (needed for Python MCP servers)
	@command -v uvx >/dev/null 2>&1 || { echo "$(YELLOW)Warning: uvx not found. Install uv for Python MCP servers.$(NC)"; }
	@# Build agentgateway if needed
	@if [ ! -f "$(PROJECT_ROOT)/target/release/agentgateway" ]; then \
		echo "Building agentgateway..."; \
		cd $(PROJECT_ROOT) && make build; \
	fi
	@echo "$(GREEN)Setup complete!$(NC)"

build: ## Build agentgateway from source
	@echo "$(BLUE)Building agentgateway...$(NC)"
	cd $(PROJECT_ROOT) && make build
	@echo "$(GREEN)Build complete!$(NC)"

clean: ## Clean build artifacts
	cd $(PROJECT_ROOT) && make clean

# =============================================================================
# Services
# =============================================================================

start-services: ## Start agentgateway with demo config (foreground)
	@echo "$(BLUE)Starting AgentGateway...$(NC)"
	@echo "MCP Endpoint: http://localhost:$(GATEWAY_PORT)/mcp"
	@echo "SSE Endpoint: http://localhost:$(GATEWAY_PORT)/sse"
	@echo "Admin UI:     http://localhost:$(ADMIN_PORT)/ui"
	@echo ""
	./start_gateway.sh

start-services-bg: ## Start services in background
	@echo "$(BLUE)Starting AgentGateway in background...$(NC)"
	./start_gateway.sh > /tmp/agentgateway-demo.log 2>&1 &
	@sleep 2
	@echo "$(GREEN)Gateway started. Logs: /tmp/agentgateway-demo.log$(NC)"

stop-services: ## Stop background services
	@echo "$(BLUE)Stopping services...$(NC)"
	-pkill -f "agentgateway.*demo-config" 2>/dev/null || true
	@echo "$(GREEN)Services stopped.$(NC)"

# =============================================================================
# Interactive Demos
# =============================================================================

demo: ## Run interactive demo (all patterns)
	@echo "$(BLUE)Starting interactive demo...$(NC)"
	python3 run_demo.py

demo-multiplexing: ## Demo: MCP multiplexing pattern
	python3 run_demo.py --demo multiplexing

demo-aliasing: ## Demo: Tool aliasing pattern
	python3 run_demo.py --demo aliasing

demo-projection: ## Demo: Output projection pattern
	python3 run_demo.py --demo projection

demo-transformation: ## Demo: Output transformation pattern
	python3 run_demo.py --demo transformation

demo-composition: ## Demo: Tool composition pattern
	python3 run_demo.py --demo composition

demo-a2a: ## Demo: A2A protocol proxying
	@echo "$(BLUE)Starting A2A demo...$(NC)"
	@echo "This demo requires an A2A agent. Starting one..."
	@# Start A2A agent in background
	cd $(PROJECT_ROOT)/examples/a2a/strands-agents && \
		(uv run . > /tmp/a2a-agent.log 2>&1 &) || \
		echo "$(YELLOW)Could not start Strands agent. Try manual setup.$(NC)"
	@sleep 3
	python3 run_demo.py --demo a2a

# =============================================================================
# Framework-Specific Demos
# =============================================================================

run-claude: ## Demo with Claude Code (MCP client)
	@echo "$(BLUE)Configuring Claude Code to use AgentGateway...$(NC)"
	@echo ""
	@echo "Add this to your Claude Code MCP settings:"
	@echo ""
	@echo '  "agentgateway": {'
	@echo '    "command": "npx",'
	@echo '    "args": ["@anthropic/claude-code-mcp-client", "http://localhost:$(GATEWAY_PORT)/sse"]'
	@echo '  }'
	@echo ""
	@echo "Or connect via SSE transport:"
	@echo "  URL: http://localhost:$(GATEWAY_PORT)/sse"
	@echo ""
	@echo "Then in Claude Code, you can use tools like:"
	@echo "  - fetch (or get_webpage, browse)"
	@echo "  - get_current_time, what_day_is_it"
	@echo "  - create_entities, read_graph"
	@echo ""
	@echo "Start the gateway first: make start-services"

run-langgraph: ## Demo with LangGraph agent
	@echo "$(BLUE)LangGraph Integration Demo$(NC)"
	@echo ""
	@echo "LangGraph can connect to AgentGateway via MCP:"
	@echo ""
	@echo "from langchain_mcp import MCPToolkit"
	@echo ""
	@echo "toolkit = MCPToolkit("
	@echo '    transport="sse",'
	@echo '    url="http://localhost:$(GATEWAY_PORT)/sse"'
	@echo ")"
	@echo ""
	@echo "tools = toolkit.get_tools()"
	@echo ""
	@echo "# Use with LangGraph"
	@echo "from langgraph.prebuilt import create_react_agent"
	@echo "agent = create_react_agent(llm, tools)"
	@echo ""
	@if [ -f langgraph_demo.py ]; then \
		echo "Running LangGraph demo..."; \
		python3 langgraph_demo.py; \
	else \
		echo "$(YELLOW)langgraph_demo.py not found. Create it to run a full demo.$(NC)"; \
	fi

run-adk: ## Demo with Google Agent Development Kit
	@echo "$(BLUE)Google ADK Integration Demo$(NC)"
	@echo ""
	@echo "Google ADK can connect to AgentGateway via A2A protocol:"
	@echo ""
	@echo "from google.adk import AgentClient"
	@echo ""
	@echo "client = AgentClient("
	@echo '    agent_url="http://localhost:$(GATEWAY_PORT)/a2a"'
	@echo ")"
	@echo ""
	@echo "# Get agent capabilities"
	@echo "card = client.get_agent_card()"
	@echo ""
	@echo "# Send message"
	@echo 'response = client.send_message("Hello, agent!")'
	@echo ""
	@if [ -f adk_demo.py ]; then \
		echo "Running ADK demo..."; \
		python3 adk_demo.py; \
	else \
		echo "$(YELLOW)adk_demo.py not found. Create it to run a full demo.$(NC)"; \
	fi

# =============================================================================
# Docker
# =============================================================================

docker-up: ## Start all services via Docker Compose
	@echo "$(BLUE)Starting Docker services...$(NC)"
	docker-compose up -d
	@echo "$(GREEN)Services started!$(NC)"
	@echo "Gateway: http://localhost:$(GATEWAY_PORT)"
	@echo "Admin UI: http://localhost:$(ADMIN_PORT)/ui"

docker-down: ## Stop Docker services
	@echo "$(BLUE)Stopping Docker services...$(NC)"
	docker-compose down

docker-logs: ## Follow Docker logs
	docker-compose logs -f

# =============================================================================
# Utilities
# =============================================================================

check: ## Check if gateway is running
	@curl -s http://localhost:$(GATEWAY_PORT)/health > /dev/null 2>&1 && \
		echo "$(GREEN)Gateway is running$(NC)" || \
		echo "$(YELLOW)Gateway not reachable$(NC)"

list-tools: ## List available tools from gateway
	@echo "$(BLUE)Fetching tools from gateway...$(NC)"
	@curl -s -X POST http://localhost:$(GATEWAY_PORT)/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | \
		python3 -c "import sys,json; tools=json.load(sys.stdin).get('result',{}).get('tools',[]); print('\n'.join(t['name'] for t in tools))"

inspector: ## Open MCP Inspector
	@echo "$(BLUE)Starting MCP Inspector...$(NC)"
	npx @modelcontextprotocol/inspector http://localhost:$(GATEWAY_PORT)/mcp
