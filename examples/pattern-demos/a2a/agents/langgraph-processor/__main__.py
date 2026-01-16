"""LangGraph Processor A2A Agent - Entry point."""

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
AGENT_NAME = "LangGraph Processor"
AGENT_PORT = 9002
ADK_URL = "http://localhost:9003"  # Google ADK Specialist

# Agent card following A2A spec
AGENT_CARD = {
    "name": AGENT_NAME,
    "description": "Workflow agent that handles multi-step processing with state management",
    "url": f"http://localhost:{AGENT_PORT}",
    "version": "0.1.0",
    "protocolVersion": "0.3.0",
    "capabilities": {
        "streaming": True,
        "pushNotifications": False,
        "stateTransitionHistory": True,
    },
    "defaultInputModes": ["text"],
    "defaultOutputModes": ["text"],
    "skills": [
        {
            "id": "workflow-execution",
            "name": "Workflow Execution",
            "description": "Executes multi-step workflows with state persistence",
            "tags": ["workflow", "state", "processing"],
            "examples": ["Execute this multi-step workflow"],
            "inputModes": ["text"],
            "outputModes": ["text"],
        },
        {
            "id": "data-transformation",
            "name": "Data Transformation",
            "description": "Transforms and processes data through defined pipelines",
            "tags": ["data", "transformation", "pipeline"],
            "examples": ["Transform this data through the pipeline"],
            "inputModes": ["text", "data"],
            "outputModes": ["text", "data"],
        },
    ],
}

# Simulated workflow state
workflow_states: dict[str, dict] = {}


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


async def delegate_to_adk(message_text: str) -> dict:
    """Delegate a task to the Google ADK Specialist agent."""
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
            response = await client.post(ADK_URL, json=request_body)
            response.raise_for_status()
            return response.json()
        except httpx.HTTPError as e:
            logger.error(f"Failed to delegate to ADK: {e}")
            return {"error": str(e)}


def analyze_workflow(message_text: str) -> tuple[list[str], bool]:
    """
    Analyze the message and determine workflow steps.
    Returns (workflow_steps, needs_adk_delegation).
    """
    message_lower = message_text.lower()

    # Define workflow steps based on content
    steps = []
    needs_adk = False

    # Step 1: Always parse input
    steps.append("Parse and validate input data")

    # Step 2: Check for transformation needs
    if any(word in message_lower for word in ["transform", "convert", "process", "pipeline"]):
        steps.append("Apply data transformation pipeline")

    # Step 3: Check for GCP/cloud needs -> delegate to ADK
    if any(word in message_lower for word in ["gcp", "google", "vertex", "cloud", "bigquery"]):
        steps.append("Delegate to ADK Specialist for cloud operations")
        needs_adk = True

    # Step 4: Always finalize
    steps.append("Aggregate results and prepare response")

    return steps, needs_adk


async def execute_workflow(task_id: str, message_text: str) -> str:
    """Execute the workflow and return results."""
    steps, needs_adk = analyze_workflow(message_text)

    # Initialize workflow state
    workflow_states[task_id] = {
        "steps": steps,
        "current_step": 0,
        "results": [],
        "status": "running",
    }

    state = workflow_states[task_id]
    results = []

    for i, step in enumerate(steps):
        state["current_step"] = i
        logger.info(f"Workflow {task_id}: Executing step {i+1}/{len(steps)} - {step}")

        if "Delegate to ADK" in step and needs_adk:
            # Delegate to ADK Specialist
            logger.info(f"Workflow {task_id}: Delegating to ADK Specialist")
            adk_result = await delegate_to_adk(f"Cloud operation for: {message_text}")

            if "error" in adk_result:
                results.append(f"Step {i+1} (ADK): Error - {adk_result['error']}")
            else:
                # Extract ADK response
                result = adk_result.get("result", {})
                if isinstance(result, dict):
                    status = result.get("status", {})
                    status_msg = status.get("message", {})
                    content = status_msg.get("content", [])
                    adk_text = ""
                    for part in content:
                        if part.get("kind") == "Text":
                            adk_text += part.get("text", "")
                    results.append(f"Step {i+1} (ADK): {adk_text}")
                else:
                    results.append(f"Step {i+1} (ADK): {result}")
        else:
            # Simulate step execution
            results.append(f"Step {i+1}: {step} - Completed")

        state["results"].append(results[-1])

    state["status"] = "completed"

    # Format response
    response_lines = [
        "[LangGraph Processor] Workflow Execution Report",
        f"Task ID: {task_id}",
        f"Total Steps: {len(steps)}",
        "",
        "Execution Log:",
    ]
    response_lines.extend(f"  - {r}" for r in results)

    return "\n".join(response_lines)


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
    elif method == "tasks/get":
        return await handle_get_task(request_id, params)
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

    # Execute the workflow
    logger.info(f"Starting workflow execution for task {task_id}")
    response_text = await execute_workflow(task_id, message_text)

    task_response = create_task_response(task_id, context_id, "completed", response_text)

    return JSONResponse({"jsonrpc": "2.0", "id": request_id, "result": task_response})


async def handle_get_task(request_id: str, params: dict) -> Response:
    """Handle tasks/get requests to check workflow status."""
    task_id = params.get("id", "")

    if task_id not in workflow_states:
        return JSONResponse(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32000, "message": f"Task not found: {task_id}"},
            },
            status_code=404,
        )

    state = workflow_states[task_id]
    context_id = str(uuid.uuid4().hex[:32])

    task_response = create_task_response(
        task_id,
        context_id,
        state["status"],
        f"Step {state['current_step'] + 1}/{len(state['steps'])}: {state['steps'][state['current_step']]}",
    )

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
