"""Google ADK Specialist A2A Agent - Entry point."""

import json
import logging
import uuid
from datetime import datetime

from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import JSONResponse, Response
from starlette.routing import Route

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Agent configuration
AGENT_NAME = "Google ADK Specialist"
AGENT_PORT = 9003

# Agent card following A2A spec
AGENT_CARD = {
    "name": AGENT_NAME,
    "description": "Specialized agent for Google Cloud operations and AI services",
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
            "id": "cloud-operations",
            "name": "Cloud Operations",
            "description": "Performs Google Cloud infrastructure operations",
            "tags": ["gcp", "cloud", "infrastructure"],
            "examples": ["List GCS buckets", "Query BigQuery table"],
            "inputModes": ["text"],
            "outputModes": ["text", "data"],
        },
        {
            "id": "vertex-ai",
            "name": "Vertex AI Integration",
            "description": "Integrates with Vertex AI for ML operations",
            "tags": ["ai", "ml", "vertex"],
            "examples": ["Run inference on Vertex AI model", "List deployed endpoints"],
            "inputModes": ["text", "data"],
            "outputModes": ["text", "data"],
        },
        {
            "id": "bigquery",
            "name": "BigQuery Analysis",
            "description": "Executes BigQuery queries and returns results",
            "tags": ["bigquery", "sql", "analytics"],
            "examples": ["Query sales data from BigQuery"],
            "inputModes": ["text"],
            "outputModes": ["text", "data"],
        },
    ],
}

# Simulated tool results for demo purposes
SIMULATED_TOOLS = {
    "gcs": {
        "description": "Google Cloud Storage operations",
        "result": {
            "buckets": ["my-project-data", "my-project-models", "my-project-logs"],
            "status": "success",
        },
    },
    "bigquery": {
        "description": "BigQuery data warehouse operations",
        "result": {
            "query_status": "completed",
            "rows_processed": 1500000,
            "bytes_processed": "2.5 GB",
            "sample_results": [
                {"date": "2026-01-15", "revenue": 125000, "orders": 450},
                {"date": "2026-01-14", "revenue": 118500, "orders": 423},
            ],
        },
    },
    "vertex": {
        "description": "Vertex AI ML operations",
        "result": {
            "model": "gemini-1.5-pro",
            "endpoint": "projects/my-project/locations/us-central1/endpoints/12345",
            "status": "deployed",
            "latency_p50_ms": 125,
            "latency_p99_ms": 450,
        },
    },
    "pubsub": {
        "description": "Pub/Sub messaging operations",
        "result": {
            "topic": "projects/my-project/topics/events",
            "messages_published": 100,
            "status": "success",
        },
    },
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


def detect_and_execute_tools(message_text: str) -> list[dict]:
    """Detect which tools are needed and return simulated results."""
    message_lower = message_text.lower()
    results = []

    # Check for each tool
    if any(word in message_lower for word in ["gcs", "bucket", "storage", "gs://"]):
        results.append({"tool": "gcs", **SIMULATED_TOOLS["gcs"]})

    if any(word in message_lower for word in ["bigquery", "bq", "sql", "query", "analytics"]):
        results.append({"tool": "bigquery", **SIMULATED_TOOLS["bigquery"]})

    if any(word in message_lower for word in ["vertex", "model", "ml", "inference", "endpoint"]):
        results.append({"tool": "vertex", **SIMULATED_TOOLS["vertex"]})

    if any(word in message_lower for word in ["pubsub", "message", "publish", "subscribe"]):
        results.append({"tool": "pubsub", **SIMULATED_TOOLS["pubsub"]})

    # Default to vertex if nothing specific matched
    if not results:
        results.append({"tool": "vertex", **SIMULATED_TOOLS["vertex"]})

    return results


def format_tool_results(results: list[dict]) -> str:
    """Format tool execution results as a readable response."""
    lines = [
        "[Google ADK Specialist] Cloud Operations Report",
        f"Tools Executed: {len(results)}",
        "",
    ]

    for r in results:
        tool_name = r["tool"].upper()
        description = r["description"]
        result = r["result"]

        lines.append(f"--- {tool_name} ---")
        lines.append(f"Operation: {description}")
        lines.append(f"Result: {json.dumps(result, indent=2)}")
        lines.append("")

    lines.append("All operations completed successfully.")
    return "\n".join(lines)


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

    # Detect and execute tools
    logger.info(f"Processing request for task {task_id}: {message_text[:100]}")
    tool_results = detect_and_execute_tools(message_text)

    for r in tool_results:
        logger.info(f"Executed tool: {r['tool']}")

    # Format response
    response_text = format_tool_results(tool_results)

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
