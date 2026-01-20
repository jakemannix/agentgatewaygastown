"""A2A server base implementation for ecommerce agents."""

import json
import logging
import uuid
from datetime import datetime
from typing import Any, Callable, Optional

from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import JSONResponse, Response
from starlette.routing import Route

logger = logging.getLogger(__name__)


def create_task_response(
    task_id: str,
    context_id: str,
    state: str,
    message: str,
    artifacts: Optional[list] = None,
) -> dict:
    """Create a standard A2A task response."""
    return {
        "id": task_id,
        "contextId": context_id,
        "kind": "task",
        "status": {
            "state": state,
            "message": {
                "role": "agent",
                "content": [{"kind": "Text", "text": message}],
            },
            "timestamp": datetime.utcnow().isoformat(),
        },
        "artifacts": artifacts or [],
        "history": [],
    }


def create_error_response(request_id: str, code: int, message: str) -> dict:
    """Create a JSON-RPC error response."""
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": code, "message": message},
    }


class A2AServer:
    """Base A2A server implementation."""

    def __init__(
        self,
        name: str,
        description: str,
        port: int,
        skills: list[dict],
        version: str = "0.1.0",
        protocol_version: str = "0.3.0",
    ):
        self.name = name
        self.description = description
        self.port = port
        self.version = version
        self.protocol_version = protocol_version
        self.skills = skills

        # Task state storage
        self.tasks: dict[str, dict] = {}

        # Message handler (to be set by subclass)
        self._message_handler: Optional[Callable] = None

        # Build agent card
        self.agent_card = {
            "name": name,
            "description": description,
            "url": f"http://localhost:{port}",
            "version": version,
            "protocolVersion": protocol_version,
            "capabilities": {
                "streaming": True,
                "pushNotifications": False,
                "stateTransitionHistory": True,
            },
            "defaultInputModes": ["text"],
            "defaultOutputModes": ["text"],
            "skills": skills,
        }

    def set_message_handler(self, handler: Callable):
        """Set the async message handler function.

        Handler signature: async def handler(message_text: str, context: dict) -> str
        """
        self._message_handler = handler

    async def handle_agent_card(self, request: Request) -> Response:
        """Handle GET requests for agent card."""
        return JSONResponse(self.agent_card)

    async def handle_a2a_request(self, request: Request) -> Response:
        """Handle A2A JSON-RPC requests."""
        try:
            body = await request.json()
        except json.JSONDecodeError:
            return JSONResponse(
                create_error_response(None, -32700, "Parse error"),
                status_code=400,
            )

        method = body.get("method", "")
        request_id = body.get("id", str(uuid.uuid4().hex[:32]))
        params = body.get("params", {})

        logger.info(f"[{self.name}] A2A request: method={method}, id={request_id}")

        if method in ("message/send", "message/stream"):
            return await self._handle_message(request_id, params)
        elif method == "tasks/get":
            return await self._handle_get_task(request_id, params)
        elif method == "tasks/cancel":
            return await self._handle_cancel_task(request_id, params)
        else:
            return JSONResponse(
                create_error_response(request_id, -32601, f"Method not found: {method}"),
                status_code=400,
            )

    async def _handle_message(self, request_id: str, params: dict) -> Response:
        """Handle message/send and message/stream requests."""
        message = params.get("message", {})
        content = message.get("content", [])

        # Extract text from message parts
        message_text = ""
        for part in content:
            if part.get("kind") == "Text":
                message_text += part.get("text", "")

        if not message_text:
            return JSONResponse(
                create_error_response(request_id, -32602, "No text content in message"),
                status_code=400,
            )

        task_id = str(uuid.uuid4().hex[:32])
        context_id = params.get("contextId", str(uuid.uuid4().hex[:32]))

        # Store task state
        self.tasks[task_id] = {
            "context_id": context_id,
            "message": message_text,
            "status": "running",
            "created_at": datetime.utcnow().isoformat(),
        }

        try:
            if self._message_handler:
                logger.info(f"[{self.name}] Processing message for task {task_id}")
                response_text = await self._message_handler(
                    message_text,
                    {"task_id": task_id, "context_id": context_id},
                )
                self.tasks[task_id]["status"] = "completed"
                self.tasks[task_id]["response"] = response_text
            else:
                response_text = f"[{self.name}] No message handler configured."
                self.tasks[task_id]["status"] = "failed"

            task_response = create_task_response(
                task_id, context_id, self.tasks[task_id]["status"], response_text
            )

            return JSONResponse({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": task_response,
            })

        except Exception as e:
            logger.error(f"[{self.name}] Error processing message: {e}")
            self.tasks[task_id]["status"] = "failed"
            self.tasks[task_id]["error"] = str(e)

            task_response = create_task_response(
                task_id, context_id, "failed", f"Error: {str(e)}"
            )

            return JSONResponse({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": task_response,
            })

    async def _handle_get_task(self, request_id: str, params: dict) -> Response:
        """Handle tasks/get requests."""
        task_id = params.get("id", "")

        if task_id not in self.tasks:
            return JSONResponse(
                create_error_response(request_id, -32000, f"Task not found: {task_id}"),
                status_code=404,
            )

        task = self.tasks[task_id]
        task_response = create_task_response(
            task_id,
            task["context_id"],
            task["status"],
            task.get("response", f"Status: {task['status']}"),
        )

        return JSONResponse({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": task_response,
        })

    async def _handle_cancel_task(self, request_id: str, params: dict) -> Response:
        """Handle tasks/cancel requests."""
        task_id = params.get("id", "")

        if task_id not in self.tasks:
            return JSONResponse(
                create_error_response(request_id, -32000, f"Task not found: {task_id}"),
                status_code=404,
            )

        self.tasks[task_id]["status"] = "cancelled"

        return JSONResponse({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {"cancelled": True, "id": task_id},
        })

    async def handle_root(self, request: Request) -> Response:
        """Handle root requests - POST for A2A, GET for agent card."""
        if request.method == "GET":
            return await self.handle_agent_card(request)
        elif request.method == "POST":
            return await self.handle_a2a_request(request)
        else:
            return JSONResponse({"error": "Method not allowed"}, status_code=405)

    async def handle_chat(self, request: Request) -> Response:
        """Handle simple REST /chat endpoint for user-facing chat.

        This is a simple REST API for web UIs to talk to agents.
        Not the same as A2A protocol which is for agent-to-agent communication.

        Expected JSON body:
        {
            "message": "user message text",
            "user_id": "optional user identifier",
            "session_id": "optional session identifier"
        }

        Returns:
        {
            "response": "agent response text",
            "session_id": "session identifier"
        }
        """
        try:
            body = await request.json()
        except json.JSONDecodeError:
            return JSONResponse(
                {"error": "Invalid JSON"},
                status_code=400,
            )

        message = body.get("message", "")
        if not message:
            return JSONResponse(
                {"error": "Message is required"},
                status_code=400,
            )

        user_id = body.get("user_id", "default-user")
        session_id = body.get("session_id", str(uuid.uuid4().hex[:16]))

        logger.info(f"[{self.name}] Chat request: session={session_id}, message={message[:50]}...")

        try:
            if self._message_handler:
                response_text = await self._message_handler(
                    message,
                    {"user_id": user_id, "session_id": session_id},
                )
            else:
                response_text = f"[{self.name}] No message handler configured."

            return JSONResponse({
                "response": response_text,
                "session_id": session_id,
            })

        except Exception as e:
            logger.error(f"[{self.name}] Error in chat: {e}")
            return JSONResponse(
                {"error": str(e), "session_id": session_id},
                status_code=500,
            )

    def create_app(self) -> Starlette:
        """Create the Starlette application."""
        routes = [
            Route("/", self.handle_root, methods=["GET", "POST"]),
            Route("/.well-known/agent.json", self.handle_agent_card, methods=["GET"]),
            Route("/.well-known/agent-card.json", self.handle_agent_card, methods=["GET"]),
            # REST endpoint for user-facing chat (separate from A2A)
            Route("/chat", self.handle_chat, methods=["POST"]),
        ]
        return Starlette(routes=routes)

    def run(self, host: str = "0.0.0.0"):
        """Run the A2A server."""
        import uvicorn

        logger.info(f"Starting {self.name} on port {self.port}")
        logger.info(f"Agent card: http://localhost:{self.port}/.well-known/agent.json")

        app = self.create_app()
        uvicorn.run(app, host=host, port=self.port, log_level="info")
