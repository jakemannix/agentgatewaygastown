#!/bin/bash
# Stop all eCommerce demo services
#
# This script stops all services started by start_services.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Stopping eCommerce demo services...${NC}"

# Kill tmux session if it exists
if tmux has-session -t ecommerce-demo 2>/dev/null; then
    tmux kill-session -t ecommerce-demo
    echo -e "${GREEN}Stopped tmux session 'ecommerce-demo'${NC}"
else
    echo -e "${YELLOW}No tmux session 'ecommerce-demo' found${NC}"
fi

# Also kill any orphaned processes on the known ports
for port in 8001 8002 8003 8004 8005 9001 9002 8080; do
    pid=$(lsof -ti :$port 2>/dev/null || true)
    if [ -n "$pid" ]; then
        kill $pid 2>/dev/null || true
        echo -e "  Killed process on port $port (PID: $pid)"
    fi
done

echo -e "${GREEN}All services stopped.${NC}"
