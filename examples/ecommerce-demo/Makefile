.PHONY: install seed run-gateway run-customer run-merchandiser run-all clean reset-db help

# Default target
help:
	@echo "eCommerce Demo - Makefile targets"
	@echo ""
	@echo "Setup:"
	@echo "  install       Install Python dependencies"
	@echo "  seed          Seed databases with sample data"
	@echo "  reset-db      Delete databases and re-seed"
	@echo ""
	@echo "Run:"
	@echo "  run-gateway      Start the agentgateway"
	@echo "  run-customer     Start customer web UI (port 8080)"
	@echo "  run-merchandiser Start merchandiser web UI (port 8081)"
	@echo "  run-all          Start all services (use tmux or separate terminals)"
	@echo ""
	@echo "Cleanup:"
	@echo "  clean         Remove generated files and caches"

# Install dependencies
install:
	uv sync

# Seed databases
seed:
	cd data && python seed_data.py

# Reset and re-seed databases
reset-db:
	cd data && python seed_data.py --reset

# Run agentgateway
run-gateway:
	cd ../.. && ./target/release/agentgateway -f examples/ecommerce-demo/gateway-configs/config.yaml

# Run customer UI
run-customer:
	PYTHONPATH=mcp-tools python web-ui/customer_app.py

# Run merchandiser UI
run-merchandiser:
	PYTHONPATH=mcp-tools MERCHANDISER_PORT=8081 python web-ui/merchandiser_app.py

# Run all services (prints instructions)
run-all:
	@echo "To run all services, open separate terminals and run:"
	@echo ""
	@echo "Terminal 1 - Gateway:"
	@echo "  make run-gateway"
	@echo ""
	@echo "Terminal 2 - Customer UI:"
	@echo "  make run-customer"
	@echo ""
	@echo "Terminal 3 - Merchandiser UI:"
	@echo "  make run-merchandiser"
	@echo ""
	@echo "Or use the start_services.sh script with tmux."

# Clean up
clean:
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	rm -rf .pytest_cache .ruff_cache 2>/dev/null || true
