"""Claude Delegator A2A Agent - Entry point."""

import asyncio
import json
import logging
import uuid
from datetime import datetime
from typing import Any

import httpx
from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import JSONResponse, Response
from starlette.routing import Route

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Agent configuration
AGENT_NAME = "Claude Delegator"
AGENT_PORT = 9001
LANGGRAPH_URL = "http://localhost:9002"  # LangGraph Processor

# Agent card following A2A spec
AGENT_CARD = {
    "name": AGENT_NAME,
    "description": "Orchestrating agent that analyzes tasks and delegates to specialized agents",
    "url": f"http://localhost:{AGENT_PORT}",
    "version": "0.1.0",
    "protocolVersion": "0.3.0",
    "capabilities": {
        "streaming": True,
        "pushNotifications": False,
    },
    "defaultInputModes": ["text"],
    "defaultOutputModes": ["text"],
    "skills": [
        {
            "id": "task-analysis",
            "name": "Task Analysis",
            "description": "Analyzes incoming tasks and determines optimal delegation strategy",
            "tags": ["orchestration", "planning", "delegation"],
            "examples": ["Analyze and route this workflow task"],
            "inputModes": ["text"],
            "outputModes": ["text"],
        },
        {
            "id": "result-synthesis",
            "name": "Result Synthesis",
            "description": "Combines results from multiple agents into coherent response",
            "tags": ["synthesis", "aggregation"],
            "examples": ["Combine these results into a summary"],
            "inputModes": ["text"],
            "outputModes": ["text"],
        },
    ],
}


def create_task_response(task_id: str, context_id: str, state: str, message: str) -> dict:
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
        "artifacts": [],
        "history": [],
    }


async def delegate_to_langgraph(message_text: str) -> dict:
    """Delegate a task to the LangGraph Processor agent."""
    task_id = str(uuid.uuid4().hex[:32])

    request_body = {
        "jsonrpc": "2.0",
        "id": task_id,
        "method": "message/send",
        "params": {
            "message": {
                "role": "user",
                "content": [{"kind": "Text", "text": message_text}],
            },
        },
    }

    async with httpx.AsyncClient(timeout=30.0) as client:
        try:
            response = await client.post(LANGGRAPH_URL, json=request_body)
            response.raise_for_status()
            return response.json()
        except httpx.HTTPError as e:
            logger.error(f"Failed to delegate to LangGraph: {e}")
            return {"error": str(e)}


def analyze_and_route(message_text: str) -> tuple[str, bool]:
    """
    Analyze the message and determine if delegation is needed.
    Returns (analysis, should_delegate).
    """
    message_lower = message_text.lower()

    # Check if this needs workflow processing or GCP operations
    needs_delegation = any(
        keyword in message_lower
        for keyword in ["workflow", "process", "gcp", "google", "vertex", "pipeline", "transform"]
    )

    analysis = f"Analyzed task: '{message_text[:50]}...'" if len(message_text) > 50 else f"Analyzed task: '{message_text}'"

    if needs_delegation:
        analysis += " | Decision: Delegating to LangGraph Processor for workflow execution."
    else:
        analysis += " | Decision: Handling directly (simple task)."

    return analysis, needs_delegation


async def handle_agent_card(request: Request) -> Response:
    """Handle GET requests for agent card."""
    return JSONResponse(AGENT_CARD)


async def handle_a2a_request(request: Request) -> Response:
    """Handle A2A JSON-RPC requests."""
    try:
        body = await request.json()
    except json.JSONDecodeError:
        return JSONResponse(
            {"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}},
            status_code=400,
        )

    method = body.get("method", "")
    request_id = body.get("id", str(uuid.uuid4().hex[:32]))
    params = body.get("params", {})

    logger.info(f"Received A2A request: method={method}, id={request_id}")

    if method in ("message/send", "message/stream"):
        return await handle_message(request_id, params)
    else:
        return JSONResponse(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32601, "message": f"Method not found: {method}"},
            },
            status_code=400,
        )


async def handle_message(request_id: str, params: dict) -> Response:
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
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32602, "message": "No text content in message"},
            },
            status_code=400,
        )

    task_id = str(uuid.uuid4().hex[:32])
    context_id = str(uuid.uuid4().hex[:32])

    # Analyze the message and decide on routing
    analysis, should_delegate = analyze_and_route(message_text)
    logger.info(analysis)

    if should_delegate:
        # Delegate to LangGraph Processor
        logger.info(f"Delegating task {task_id} to LangGraph Processor")
        delegate_result = await delegate_to_langgraph(message_text)

        if "error" in delegate_result:
            response_text = f"{analysis}\n\nDelegation failed: {delegate_result['error']}"
        else:
            # Extract the response from LangGraph
            result = delegate_result.get("result", {})
            if isinstance(result, dict):
                status = result.get("status", {})
                status_msg = status.get("message", {})
                content = status_msg.get("content", [])
                delegate_text = ""
                for part in content:
                    if part.get("kind") == "Text":
                        delegate_text += part.get("text", "")
                response_text = f"[Claude Delegator] {analysis}\n\n[LangGraph Response] {delegate_text}"
            else:
                response_text = f"[Claude Delegator] {analysis}\n\n[LangGraph Response] {result}"
    else:
        # Handle directly
        response_text = f"[Claude Delegator] {analysis}\n\nDirect response: Task acknowledged and processed locally."

    task_response = create_task_response(task_id, context_id, "completed", response_text)

    return JSONResponse({"jsonrpc": "2.0", "id": request_id, "result": task_response})


async def handle_root(request: Request) -> Response:
    """Handle root requests - POST for A2A, GET for agent card."""
    if request.method == "GET":
        return await handle_agent_card(request)
    elif request.method == "POST":
        return await handle_a2a_request(request)
    else:
        return JSONResponse({"error": "Method not allowed"}, status_code=405)


# Starlette routes
routes = [
    Route("/", handle_root, methods=["GET", "POST"]),
    Route("/.well-known/agent.json", handle_agent_card, methods=["GET"]),
    Route("/.well-known/agent-card.json", handle_agent_card, methods=["GET"]),
]

app = Starlette(routes=routes)


if __name__ == "__main__":
    import uvicorn
    logger.info(f"Starting {AGENT_NAME} on port {AGENT_PORT}")
    logger.info(f"Agent card available at http://localhost:{AGENT_PORT}/.well-known/agent.json")
    uvicorn.run(app, host="0.0.0.0", port=AGENT_PORT, log_level="info")
