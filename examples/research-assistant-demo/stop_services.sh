#!/bin/bash
# Stop all services for the Research Assistant demo

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo -e "${GREEN}Stopping Research Assistant Demo services...${NC}"

# Kill tmux session
if tmux has-session -t research-demo 2>/dev/null; then
    tmux kill-session -t research-demo
    echo -e "${GREEN}Stopped tmux session 'research-demo'${NC}"
else
    echo -e "${RED}No tmux session 'research-demo' found${NC}"
fi

# Also kill any orphaned processes on the ports
for port in 8001 8002 8003 8004 8005 3000 9001; do
    pid=$(lsof -t -i:$port 2>/dev/null || true)
    if [ -n "$pid" ]; then
        echo "Killing process on port $port (PID: $pid)"
        kill $pid 2>/dev/null || true
    fi
done

echo -e "\n${GREEN}All services stopped.${NC}"
