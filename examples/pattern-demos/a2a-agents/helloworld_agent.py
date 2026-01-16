#!/usr/bin/env python3
"""
Simple A2A HelloWorld Agent for pattern demos.

This is a minimal A2A agent that responds to messages with a greeting.
Used by docker-compose to demonstrate A2A protocol proxying through AgentGateway.

Usage:
    python -m helloworld_agent
    # Or: python helloworld_agent.py

The agent listens on port 9999 (configurable via PORT env var).
"""

import json
import os
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Any

PORT = int(os.environ.get("PORT", 9999))


class A2AHandler(BaseHTTPRequestHandler):
    """Simple A2A protocol handler."""

    def _send_json(self, data: dict, status: int = 200):
        """Send JSON response."""
        body = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", len(body))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        """Handle GET requests - agent card endpoint."""
        if self.path == "/.well-known/agent.json":
            self._send_json({
                "name": "HelloWorld Agent",
                "description": "A simple greeting agent for AgentGateway demos",
                "url": f"http://localhost:{PORT}",
                "version": "1.0.0",
                "capabilities": {
                    "streaming": False,
                    "pushNotifications": False
                },
                "skills": [
                    {
                        "id": "greeting",
                        "name": "Greeting",
                        "description": "Responds with a friendly greeting"
                    }
                ]
            })
        else:
            self._send_json({"error": "Not found"}, 404)

    def do_POST(self):
        """Handle POST requests - A2A messages."""
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length)

        try:
            request = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self._send_json({"error": "Invalid JSON"}, 400)
            return

        # Handle A2A JSON-RPC style request
        method = request.get("method", "")
        params = request.get("params", {})

        if method == "message/send":
            # Extract message content
            message = params.get("message", {})
            content = message.get("content", "")

            # Simple response
            response_text = f"Hello! You said: {content}"

            self._send_json({
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "result": {
                    "message": {
                        "role": "agent",
                        "content": response_text,
                        "parts": [
                            {"type": "text", "text": response_text}
                        ]
                    }
                }
            })
        else:
            self._send_json({
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "error": {
                    "code": -32601,
                    "message": f"Unknown method: {method}"
                }
            }, 400)

    def log_message(self, format: str, *args: Any):
        """Log to stdout."""
        print(f"[A2A Agent] {args[0]} {args[1]} {args[2]}")


def main():
    """Start the A2A agent server."""
    server = HTTPServer(("0.0.0.0", PORT), A2AHandler)
    print(f"HelloWorld A2A Agent running on http://0.0.0.0:{PORT}")
    print(f"Agent card: http://localhost:{PORT}/.well-known/agent.json")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()
