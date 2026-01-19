# AgentGateway Pattern Demos Makefile
#
# Orchestrates the demo environment with targets for different agent frameworks.
#
# Quick start:
#   make setup              # Install Python dependencies
#   make start-mcp-services # Start custom MCP services (document, task, user, notification)
#   make start-services     # Start gateway (aggregates all MCP servers)
#   make demo               # Run interactive demo
#
# All-in-one:
#   make start-all          # Start everything (custom services + gateway)
#   make stop-all           # Stop everything
#
# Framework-specific demos:
#   make run-claude     # Demo with Claude Code
#   make run-langgraph  # Demo with LangGraph
#   make run-adk        # Demo with Google ADK
#   make demo-a2a       # Demo A2A protocol

.PHONY: help setup start-services stop-services demo demo-debug \
        start-mcp-services stop-mcp-services check-mcp-services \
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
	@echo "Quick Start:"
	@echo "  make setup              # Install Python dependencies"
	@echo "  make start-mcp-services # Start custom MCP services (SQLite-backed)"
	@echo "  make start-services     # Start gateway (connects to all MCP servers)"
	@echo "  make demo               # Run interactive demo"
	@echo ""
	@echo "Setup & Build:"
	@grep -E '^(setup|build-all|build-ui|build|clean):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
	@echo ""
	@echo "Services:"
	@grep -E '^(start-all|stop-all|start-services|stop-services|start-mcp|stop-mcp|check-mcp):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
	@echo ""
	@echo "Demos:"
	@grep -E '^(demo|demo-debug|run-claude|run-langgraph|run-adk|demo-a2a):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
	@echo ""
	@echo "Docker:"
	@grep -E '^(docker-up|docker-down):' $(MAKEFILE_LIST) | \
		sed 's/:.*##/: ##/' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

# =============================================================================
# Setup & Build
# =============================================================================

setup: ## Install Python dependencies via uv
	@echo "$(BLUE)Setting up Python dependencies...$(NC)"
	@# Check for uv (required for Python dependencies)
	@command -v uv >/dev/null 2>&1 || { echo "$(YELLOW)Error: uv not found. Install from https://docs.astral.sh/uv/$(NC)"; exit 1; }
	@# Install Python dependencies via uv
	uv sync
	@echo "$(GREEN)Python setup complete!$(NC)"
	@echo ""
	@echo "Next steps:"
	@echo "  make build-all    # Build UI and agentgateway (requires Node.js, ~16GB RAM)"
	@echo "  make start-services  # Start the gateway"

build-all: build-ui build ## Build UI and agentgateway

build-ui: ## Build the admin UI (requires Node.js)
	@command -v npm >/dev/null 2>&1 || { echo "$(YELLOW)Error: npm not found. Install Node.js.$(NC)"; exit 1; }
	@if [ ! -d "$(PROJECT_ROOT)/ui/out" ]; then \
		echo "$(BLUE)Building UI...$(NC)"; \
		cd $(PROJECT_ROOT)/ui && npm install && npm run build; \
		echo "$(GREEN)UI build complete!$(NC)"; \
	else \
		echo "UI already built ($(PROJECT_ROOT)/ui/out exists)"; \
	fi

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
# Custom MCP Services (document, task, user, notification)
# =============================================================================

start-mcp-services: ## Start custom MCP services (document, task, user, notification)
	@echo "$(BLUE)Starting custom MCP services...$(NC)"
	./start_mcp_services.sh

stop-mcp-services: ## Stop custom MCP services
	@echo "$(BLUE)Stopping custom MCP services...$(NC)"
	./stop_mcp_services.sh

check-mcp-services: ## Check status of custom MCP services
	./start_mcp_services.sh --check

start-all: start-mcp-services start-services-bg ## Start all services (custom MCP + gateway)
	@echo "$(GREEN)All services started!$(NC)"
	@echo ""
	@echo "Endpoints:"
	@echo "  Gateway MCP:     http://localhost:$(GATEWAY_PORT)/mcp"
	@echo "  Gateway SSE:     http://localhost:$(GATEWAY_PORT)/sse"
	@echo "  Admin UI:        http://localhost:$(ADMIN_PORT)/ui"
	@echo "  Document Service: http://localhost:8001"
	@echo "  Task Service:     http://localhost:8002"
	@echo "  User Service:     http://localhost:8003"
	@echo "  Notification Svc: http://localhost:8004"

stop-all: stop-services stop-mcp-services ## Stop all services
	@echo "$(GREEN)All services stopped.$(NC)"

# =============================================================================
# Interactive Demos
# =============================================================================

demo: ## Run interactive demo (all patterns)
	@echo "$(BLUE)Starting interactive demo...$(NC)"
	@echo "$(YELLOW)Tip: Use 'make demo-debug' for verbose output$(NC)"
	uv run python run_demo.py

demo-debug: ## Run demo with debug output
	@echo "$(BLUE)Starting interactive demo (debug mode)...$(NC)"
	uv run python run_demo.py --debug

demo-multiplexing: ## Demo: MCP multiplexing pattern
	uv run python run_demo.py --demo multiplexing

demo-aliasing: ## Demo: Tool aliasing pattern
	uv run python run_demo.py --demo aliasing

demo-projection: ## Demo: Output projection pattern
	uv run python run_demo.py --demo projection

demo-transformation: ## Demo: Output transformation pattern
	uv run python run_demo.py --demo transformation

demo-composition: ## Demo: Tool composition pattern
	uv run python run_demo.py --demo composition

demo-a2a: ## Demo: A2A protocol proxying
	@echo "$(BLUE)Starting A2A demo...$(NC)"
	@echo "This demo requires an A2A agent. Starting one..."
	@# Start A2A agent in background
	cd $(PROJECT_ROOT)/examples/a2a/strands-agents && \
		(uv run . > /tmp/a2a-agent.log 2>&1 &) || \
		echo "$(YELLOW)Could not start Strands agent. Try manual setup.$(NC)"
	@sleep 3
	uv run python run_demo.py --demo a2a

# =============================================================================
# Framework-Specific Demos (Agent SDK demos using gateway tools)
# =============================================================================

run-claude-agent: ## Run Claude Agent SDK demo (requires ANTHROPIC_API_KEY)
	@echo "$(BLUE)Running Claude Agent SDK Demo$(NC)"
	@echo ""
	@if [ -z "$$ANTHROPIC_API_KEY" ]; then \
		echo "$(YELLOW)Warning: ANTHROPIC_API_KEY not set$(NC)"; \
		echo "Set it with: export ANTHROPIC_API_KEY=your-key"; \
		exit 1; \
	fi
	@echo "Available scenarios: full_workflow, document_search, task_management, user_collaboration"
	@echo ""
	cd agents/claude_agent && uv sync && uv run python -m claude_agent --scenario full_workflow

run-claude-agent-docs: ## Claude agent: document search demo
	cd agents/claude_agent && uv run python -m claude_agent --scenario document_search

run-claude-agent-tasks: ## Claude agent: task management demo  
	cd agents/claude_agent && uv run python -m claude_agent --scenario task_management

run-claude-agent-users: ## Claude agent: user collaboration demo
	cd agents/claude_agent && uv run python -m claude_agent --scenario user_collaboration

run-adk-agent: ## Run Google ADK agent demo (requires GOOGLE_API_KEY)
	@echo "$(BLUE)Running Google ADK Agent Demo$(NC)"
	@echo ""
	@if [ -z "$$GOOGLE_API_KEY" ]; then \
		echo "$(YELLOW)Warning: GOOGLE_API_KEY not set$(NC)"; \
		echo "Set it with: export GOOGLE_API_KEY=your-key"; \
		exit 1; \
	fi
	cd agents/google_adk_agent && uv sync && uv run python -m google_adk_agent

run-claude: ## Instructions for Claude Desktop / Claude Code MCP connection
	@echo "$(BLUE)Connecting Claude Desktop/Code to AgentGateway$(NC)"
	@echo ""
	@echo "Add this to your Claude Desktop MCP settings:"
	@echo "(~/Library/Application Support/Claude/claude_desktop_config.json on macOS)"
	@echo ""
	@echo '{'
	@echo '  "mcpServers": {'
	@echo '    "agentgateway": {'
	@echo '      "command": "npx",'
	@echo '      "args": ["-y", "mcp-remote", "http://localhost:$(GATEWAY_PORT)/sse"]'
	@echo '    }'
	@echo '  }'
	@echo '}'
	@echo ""
	@echo "Then restart Claude Desktop. All gateway tools will be available."
	@echo ""
	@echo "Available tools include:"
	@echo "  From custom services: create_document, search_documents, create_task,"
	@echo "                       list_users, send_notification, etc."
	@echo "  Virtual tools:       get_webpage, find_documents, add_todo, notify"
	@echo "  Pre-built servers:   fetch, get_current_time, create_entities"

run-langgraph: ## Instructions for LangGraph MCP integration
	@echo "$(BLUE)LangGraph Integration$(NC)"
	@echo ""
	@echo "Install langchain-mcp-adapters:"
	@echo "  pip install langchain-mcp-adapters"
	@echo ""
	@echo "Example code:"
	@echo ""
	@echo "from langchain_mcp_adapters import MCPToolkit"
	@echo "from langgraph.prebuilt import create_react_agent"
	@echo "from langchain_anthropic import ChatAnthropic"
	@echo ""
	@echo "# Connect to gateway"
	@echo "toolkit = MCPToolkit(server_url='http://localhost:$(GATEWAY_PORT)/sse')"
	@echo "await toolkit.initialize()"
	@echo "tools = toolkit.get_tools()"
	@echo ""
	@echo "# Create agent"
	@echo "llm = ChatAnthropic(model='claude-3-5-sonnet-latest')"
	@echo "agent = create_react_agent(llm, tools)"
	@echo ""
	@echo "# Run"
	@echo "result = await agent.ainvoke({'messages': [('user', 'Create a document about AI')]})"

run-adk: ## Instructions for Google ADK A2A integration
	@echo "$(BLUE)Google ADK Integration$(NC)"
	@echo ""
	@echo "Google ADK can connect via A2A protocol for agent-to-agent communication."
	@echo ""
	@echo "See agents/google_adk_agent/ for a complete example that:"
	@echo "  - Discovers tools from the gateway dynamically"
	@echo "  - Wraps them as ADK FunctionTools"
	@echo "  - Uses them in agent workflows"
	@echo ""
	@echo "Quick start:"
	@echo "  make run-adk-agent"

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
		uv run python -c "import sys,json; tools=json.load(sys.stdin).get('result',{}).get('tools',[]); print('\n'.join(t['name'] for t in tools))"

inspector: ## Open MCP Inspector
	@echo "$(BLUE)Starting MCP Inspector...$(NC)"
	npx @modelcontextprotocol/inspector http://localhost:$(GATEWAY_PORT)/mcp
