#!/usr/bin/env python3
"""FastHTML Chat UI with split-screen for two agents.

This provides side-by-side chat interfaces for:
- Customer Agent (shopping tasks) - left panel
- Merchandiser Agent (inventory/supply chain tasks) - right panel
"""

import logging
import os
import uuid
from pathlib import Path

import httpx
from fasthtml.common import *

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

# Configuration
CUSTOMER_AGENT_URL = os.environ.get("CUSTOMER_AGENT_URL", "http://localhost:9001")
MERCHANDISER_AGENT_URL = os.environ.get("MERCHANDISER_AGENT_URL", "http://localhost:9002")
WEB_PORT = int(os.environ.get("WEB_PORT", 8080))

# Create FastHTML app
app, rt = fast_app(
    static_path=str(Path(__file__).parent / "static"),
)

# Session storage (in-memory for demo) - separate histories per agent
sessions: dict[str, dict] = {}


def get_session(session_id: str) -> dict:
    """Get or create a session with separate message histories."""
    if session_id not in sessions:
        sessions[session_id] = {
            "customer_messages": [],
            "merchandiser_messages": [],
            "customer_chat_id": str(uuid.uuid4().hex[:16]),
            "merchandiser_chat_id": str(uuid.uuid4().hex[:16]),
        }
    return sessions[session_id]


async def send_chat_message(agent_url: str, message: str, session_id: str, user_id: str = "web-user") -> str:
    """Send a message to an agent via REST /chat endpoint."""
    request_body = {
        "message": message,
        "session_id": session_id,
        "user_id": user_id,
    }

    async with httpx.AsyncClient(timeout=120.0) as client:
        try:
            response = await client.post(f"{agent_url}/chat", json=request_body)
            response.raise_for_status()
            result = response.json()

            if "response" in result:
                return result["response"]
            elif "error" in result:
                return f"Error: {result['error']}"
            else:
                return "Unexpected response format."

        except httpx.HTTPError as e:
            logger.error(f"HTTP error calling agent: {e}")
            return f"Error communicating with agent: {str(e)}"
        except Exception as e:
            logger.error(f"Error calling agent: {e}")
            return f"Error: {str(e)}"


async def get_agent_card(agent_url: str) -> dict:
    """Fetch agent card from A2A endpoint."""
    async with httpx.AsyncClient(timeout=10.0) as client:
        try:
            response = await client.get(f"{agent_url}/.well-known/agent.json")
            response.raise_for_status()
            return response.json()
        except Exception as e:
            logger.error(f"Error fetching agent card: {e}")
            return {}


def render_message(role: str, content: str) -> Div:
    """Render a chat message."""
    return Div(
        Div(
            P(content, cls="message-content"),
            cls=f"message-bubble",
        ),
        cls=f"message {role}",
    )


STYLES = """
* { box-sizing: border-box; }
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 0; padding: 1rem; background: #f1f5f9;
}
h1 { text-align: center; margin: 0 0 1rem 0; color: #1e293b; }

.split-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    max-width: 1400px;
    margin: 0 auto;
}

.chat-panel {
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
    display: flex;
    flex-direction: column;
    height: calc(100vh - 100px);
    min-height: 500px;
}

.panel-header {
    padding: 1rem;
    border-bottom: 1px solid #e2e8f0;
}
.panel-header h2 { margin: 0 0 0.25rem 0; font-size: 1.1rem; }
.panel-header p { margin: 0; color: #64748b; font-size: 0.8rem; }
.panel-header.customer { border-left: 4px solid #2563eb; }
.panel-header.merchandiser { border-left: 4px solid #16a34a; }

.status-badge {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 500;
    margin-left: 0.5rem;
}
.status-badge.online { background: #dcfce7; color: #166534; }
.status-badge.offline { background: #fee2e2; color: #991b1b; }

.chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    background: #f8fafc;
}

.message { margin-bottom: 0.75rem; }
.message-bubble {
    max-width: 85%;
    padding: 0.6rem 0.9rem;
    border-radius: 12px;
    display: inline-block;
    font-size: 0.9rem;
    line-height: 1.4;
}
.message.user { text-align: right; }
.message.user .message-bubble { background: #2563eb; color: white; }
.message.agent .message-bubble { background: white; border: 1px solid #e2e8f0; }
.message-content { margin: 0; white-space: pre-wrap; word-wrap: break-word; }

.chat-form {
    padding: 0.75rem;
    border-top: 1px solid #e2e8f0;
    display: flex;
    gap: 0.5rem;
}
.chat-input {
    flex: 1;
    padding: 0.6rem 0.9rem;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    font-size: 0.9rem;
}
.chat-input:focus { outline: none; border-color: #2563eb; }

.send-btn {
    padding: 0.6rem 1.2rem;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
    font-size: 0.9rem;
}
.send-btn.customer { background: #2563eb; color: white; }
.send-btn.customer:hover { background: #1d4ed8; }
.send-btn.merchandiser { background: #16a34a; color: white; }
.send-btn.merchandiser:hover { background: #15803d; }

.quick-actions {
    padding: 0.5rem 0.75rem;
    border-top: 1px solid #e2e8f0;
    background: #f8fafc;
}
.quick-actions p { margin: 0 0 0.4rem 0; font-size: 0.75rem; color: #64748b; }
.quick-btn {
    padding: 0.3rem 0.6rem;
    margin: 0.15rem;
    background: #e2e8f0;
    border: none;
    border-radius: 6px;
    font-size: 0.75rem;
    cursor: pointer;
}
.quick-btn:hover { background: #cbd5e1; }

.htmx-request .send-btn { opacity: 0.6; cursor: wait; }
"""


@rt("/")
async def home(request):
    """Home page with split-screen chat."""
    session_id = request.cookies.get("session_id", str(uuid.uuid4())[:8])
    session = get_session(session_id)

    # Fetch agent cards
    customer_card = await get_agent_card(CUSTOMER_AGENT_URL)
    merchandiser_card = await get_agent_card(MERCHANDISER_AGENT_URL)

    content = Html(
        Head(
            Title("eCommerce Demo - Split Chat"),
            Meta(charset="utf-8"),
            Meta(name="viewport", content="width=device-width, initial-scale=1"),
            Script(src="https://unpkg.com/htmx.org@1.9.10"),
            Style(STYLES),
        ),
        Body(
            H1("eCommerce Agent Demo"),

            Div(
                # Left panel - Customer Agent
                Div(
                    Div(
                        H2(
                            "🛒 Customer Agent",
                            Span("Online" if customer_card else "Offline",
                                 cls=f"status-badge {'online' if customer_card else 'offline'}"),
                        ),
                        P(customer_card.get("description", "Shopping assistant")[:80] + "..."
                          if customer_card else "Offline"),
                        cls="panel-header customer",
                    ),
                    Div(
                        *[render_message(m["role"], m["content"]) for m in session["customer_messages"]],
                        id="customer-messages",
                        cls="chat-messages",
                    ),
                    Form(
                        Input(type="text", name="message", placeholder="Ask about products, cart, orders...",
                              autocomplete="off", cls="chat-input", id="customer-input"),
                        Input(type="hidden", name="session_id", value=session_id),
                        Input(type="hidden", name="agent", value="customer"),
                        Button("Send", type="submit", cls="send-btn customer"),
                        hx_post="/chat",
                        hx_target="#customer-messages",
                        hx_swap="beforeend",
                        hx_on_htmx_after_request="this.querySelector('input[name=message]').value = ''",
                        cls="chat-form",
                    ),
                    Div(
                        P("Quick:"),
                        Button("Search products", onclick="sendTo('customer', 'Show me popular products')", cls="quick-btn"),
                        Button("View cart", onclick="sendTo('customer', 'Show my cart')", cls="quick-btn"),
                        Button("My orders", onclick="sendTo('customer', 'Show my orders')", cls="quick-btn"),
                        cls="quick-actions",
                    ),
                    cls="chat-panel",
                ),

                # Right panel - Merchandiser Agent
                Div(
                    Div(
                        H2(
                            "📦 Merchandiser Agent",
                            Span("Online" if merchandiser_card else "Offline",
                                 cls=f"status-badge {'online' if merchandiser_card else 'offline'}"),
                        ),
                        P(merchandiser_card.get("description", "Inventory management")[:80] + "..."
                          if merchandiser_card else "Offline"),
                        cls="panel-header merchandiser",
                    ),
                    Div(
                        *[render_message(m["role"], m["content"]) for m in session["merchandiser_messages"]],
                        id="merchandiser-messages",
                        cls="chat-messages",
                    ),
                    Form(
                        Input(type="text", name="message", placeholder="Ask about inventory, orders, suppliers...",
                              autocomplete="off", cls="chat-input", id="merchandiser-input"),
                        Input(type="hidden", name="session_id", value=session_id),
                        Input(type="hidden", name="agent", value="merchandiser"),
                        Button("Send", type="submit", cls="send-btn merchandiser"),
                        hx_post="/chat",
                        hx_target="#merchandiser-messages",
                        hx_swap="beforeend",
                        hx_on_htmx_after_request="this.querySelector('input[name=message]').value = ''",
                        cls="chat-form",
                    ),
                    Div(
                        P("Quick:"),
                        Button("Inventory status", onclick="sendTo('merchandiser', 'Show inventory report')", cls="quick-btn"),
                        Button("Low stock", onclick="sendTo('merchandiser', 'What items need restocking?')", cls="quick-btn"),
                        Button("Suppliers", onclick="sendTo('merchandiser', 'List our suppliers')", cls="quick-btn"),
                        cls="quick-actions",
                    ),
                    cls="chat-panel",
                ),

                cls="split-container",
            ),

            Script("""
                function sendTo(agent, message) {
                    const input = document.getElementById(agent + '-input');
                    input.value = message;
                    input.closest('form').dispatchEvent(new Event('submit', {bubbles: true}));
                }

                // Auto-scroll both chat panels
                document.querySelectorAll('.chat-messages').forEach(el => {
                    const observer = new MutationObserver(() => {
                        el.scrollTop = el.scrollHeight;
                    });
                    observer.observe(el, { childList: true, subtree: true });
                });
            """),
        ),
    )

    # Handle FastHTML tuple return
    if isinstance(content, tuple):
        rendered = "".join(str(c) for c in content)
    else:
        rendered = str(content)

    response = Response(content=rendered, media_type="text/html")
    response.set_cookie("session_id", session_id, max_age=86400)
    return response


@rt("/chat", methods=["POST"])
async def chat(request, message: str, session_id: str, agent: str):
    """Handle chat messages for either agent."""
    logger.debug(f"Chat request: session={session_id}, agent={agent}, message={repr(message)}")

    # Validate message
    if not message or not message.strip():
        logger.warning("Empty message received, ignoring")
        return Div()

    message = message.strip()
    session = get_session(session_id)

    # Determine agent URL and message list
    if agent == "customer":
        agent_url = CUSTOMER_AGENT_URL
        messages_key = "customer_messages"
        chat_id = session["customer_chat_id"]
    else:
        agent_url = MERCHANDISER_AGENT_URL
        messages_key = "merchandiser_messages"
        chat_id = session["merchandiser_chat_id"]

    # Add user message
    session[messages_key].append({"role": "user", "content": message})

    # Send to agent
    logger.info(f"Sending to {agent} agent: {message[:50]}...")
    response_text = await send_chat_message(agent_url, message, chat_id)

    # Add agent response
    session[messages_key].append({"role": "agent", "content": response_text})

    # Return both messages
    return Div(
        render_message("user", message),
        render_message("agent", response_text),
    )


@rt("/clear/{agent}", methods=["POST"])
async def clear_chat(session_id: str, agent: str):
    """Clear chat history for a specific agent."""
    if session_id in sessions:
        if agent == "customer":
            sessions[session_id]["customer_messages"] = []
            sessions[session_id]["customer_chat_id"] = str(uuid.uuid4().hex[:16])
        else:
            sessions[session_id]["merchandiser_messages"] = []
            sessions[session_id]["merchandiser_chat_id"] = str(uuid.uuid4().hex[:16])

    return RedirectResponse("/", status_code=303)


if __name__ == "__main__":
    import uvicorn

    logger.info(f"Starting Split Chat UI on port {WEB_PORT}")
    logger.info(f"Customer Agent: {CUSTOMER_AGENT_URL}")
    logger.info(f"Merchandiser Agent: {MERCHANDISER_AGENT_URL}")

    uvicorn.run(app, host="0.0.0.0", port=WEB_PORT)
