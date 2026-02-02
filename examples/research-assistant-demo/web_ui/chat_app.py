#!/usr/bin/env python3
"""FastHTML Chat UI for Research Assistant.

A clean chat interface for interacting with the research agent.
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
RESEARCH_AGENT_URL = os.environ.get("RESEARCH_AGENT_URL", "http://localhost:9001")
WEB_PORT = int(os.environ.get("WEB_PORT", 8080))

# Create FastHTML app
app, rt = fast_app(
    static_path=str(Path(__file__).parent / "static"),
)

# Session storage (in-memory for demo)
sessions: dict[str, dict] = {}


def get_session(session_id: str) -> dict:
    """Get or create a session."""
    if session_id not in sessions:
        sessions[session_id] = {
            "messages": [],
            "chat_id": str(uuid.uuid4().hex[:16]),
        }
    return sessions[session_id]


async def send_chat_message(agent_url: str, message: str, session_id: str, user_id: str = "web-user") -> str:
    """Send a message to the agent via REST /chat endpoint."""
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
    """Render a chat message with markdown support."""
    return Div(
        Div(
            Div(content, cls="message-content"),
            cls="message-bubble",
        ),
        cls=f"message {role}",
    )


STYLES = """
* { box-sizing: border-box; }
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 0;
    padding: 0;
    background: #0f172a;
    color: #e2e8f0;
    min-height: 100vh;
}

.container {
    max-width: 900px;
    margin: 0 auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    height: 100vh;
}

header {
    text-align: center;
    padding: 1.5rem 0;
    border-bottom: 1px solid #334155;
    margin-bottom: 1rem;
}

header h1 {
    margin: 0 0 0.5rem 0;
    font-size: 1.75rem;
    background: linear-gradient(135deg, #60a5fa 0%, #a78bfa 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

header p {
    margin: 0;
    color: #94a3b8;
    font-size: 0.9rem;
}

.status-badge {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 500;
    margin-left: 0.75rem;
}
.status-badge.online { background: #166534; color: #dcfce7; }
.status-badge.offline { background: #991b1b; color: #fee2e2; }

.chat-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #1e293b;
    border-radius: 16px;
    overflow: hidden;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
}

.chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.message {
    display: flex;
    flex-direction: column;
}

.message.user {
    align-items: flex-end;
}

.message.agent {
    align-items: flex-start;
}

.message-bubble {
    max-width: 80%;
    padding: 0.875rem 1.125rem;
    border-radius: 16px;
    font-size: 0.95rem;
    line-height: 1.5;
}

.message.user .message-bubble {
    background: linear-gradient(135deg, #3b82f6 0%, #6366f1 100%);
    color: white;
    border-bottom-right-radius: 4px;
}

.message.agent .message-bubble {
    background: #334155;
    color: #e2e8f0;
    border-bottom-left-radius: 4px;
}

.message-content {
    white-space: pre-wrap;
    word-wrap: break-word;
}

.chat-input-container {
    padding: 1rem 1.5rem;
    background: #1e293b;
    border-top: 1px solid #334155;
}

.chat-form {
    display: flex;
    gap: 0.75rem;
}

.chat-input {
    flex: 1;
    padding: 0.875rem 1.25rem;
    border: 1px solid #475569;
    border-radius: 12px;
    background: #0f172a;
    color: #e2e8f0;
    font-size: 0.95rem;
    outline: none;
    transition: border-color 0.2s, box-shadow 0.2s;
}

.chat-input:focus {
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.2);
}

.chat-input::placeholder {
    color: #64748b;
}

.send-btn {
    padding: 0.875rem 1.5rem;
    border: none;
    border-radius: 12px;
    background: linear-gradient(135deg, #3b82f6 0%, #6366f1 100%);
    color: white;
    font-weight: 600;
    font-size: 0.95rem;
    cursor: pointer;
    transition: transform 0.1s, box-shadow 0.2s;
}

.send-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
}

.send-btn:active {
    transform: translateY(0);
}

.quick-actions {
    padding: 0.75rem 1.5rem 1rem;
    background: #1e293b;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.quick-actions-label {
    width: 100%;
    font-size: 0.75rem;
    color: #64748b;
    margin-bottom: 0.25rem;
}

.quick-btn {
    padding: 0.5rem 0.875rem;
    background: #334155;
    border: 1px solid #475569;
    border-radius: 8px;
    color: #e2e8f0;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s;
}

.quick-btn:hover {
    background: #475569;
    border-color: #64748b;
}

.htmx-request .send-btn {
    opacity: 0.7;
    cursor: wait;
}

.welcome-message {
    text-align: center;
    padding: 3rem 2rem;
    color: #94a3b8;
}

.welcome-message h2 {
    color: #e2e8f0;
    margin-bottom: 1rem;
}

.welcome-message ul {
    text-align: left;
    display: inline-block;
    margin: 1rem 0;
}

.welcome-message li {
    margin: 0.5rem 0;
}

/* Loading indicator */
.loading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: #94a3b8;
    font-size: 0.875rem;
    padding: 0.75rem 1rem;
}

.loading-dots {
    display: flex;
    gap: 0.25rem;
}

.loading-dots span {
    width: 6px;
    height: 6px;
    background: #64748b;
    border-radius: 50%;
    animation: bounce 1.4s infinite ease-in-out both;
}

.loading-dots span:nth-child(1) { animation-delay: -0.32s; }
.loading-dots span:nth-child(2) { animation-delay: -0.16s; }

@keyframes bounce {
    0%, 80%, 100% { transform: scale(0); }
    40% { transform: scale(1); }
}
"""


@rt("/")
async def home(request):
    """Home page with chat interface."""
    session_id = request.cookies.get("session_id", str(uuid.uuid4())[:8])
    session = get_session(session_id)

    # Fetch agent card
    agent_card = await get_agent_card(RESEARCH_AGENT_URL)
    is_online = bool(agent_card)

    content = Html(
        Head(
            Title("Research Assistant"),
            Meta(charset="utf-8"),
            Meta(name="viewport", content="width=device-width, initial-scale=1"),
            Script(src="https://unpkg.com/htmx.org@1.9.10"),
            Style(STYLES),
        ),
        Body(
            Div(
                Header(
                    H1(
                        "Research Assistant",
                        Span(
                            "Online" if is_online else "Offline",
                            cls=f"status-badge {'online' if is_online else 'offline'}"
                        ),
                    ),
                    P(agent_card.get("description", "AI-powered research assistant")[:100] if agent_card else "Agent offline"),
                ),

                Div(
                    Div(
                        # Welcome message if no messages yet
                        Div(
                            H2("Welcome to Research Assistant"),
                            P("I can help you with:"),
                            Ul(
                                Li("Searching across web, arXiv, GitHub, and HuggingFace"),
                                Li("Storing and organizing research findings"),
                                Li("Building knowledge graphs and relationships"),
                                Li("Deep dives into technical topics"),
                            ),
                            P("Try one of the quick actions below or ask me anything!"),
                            cls="welcome-message",
                        ) if not session["messages"] else None,
                        *[render_message(m["role"], m["content"]) for m in session["messages"]],
                        id="chat-messages",
                        cls="chat-messages",
                    ),

                    Div(
                        Span("Quick actions:", cls="quick-actions-label"),
                        Button("Search transformers", onclick="sendMessage('Research transformer alternatives for 2025-2026')", cls="quick-btn"),
                        Button("arXiv papers", onclick="sendMessage('Find recent arXiv papers on state space models')", cls="quick-btn"),
                        Button("GitHub repos", onclick="sendMessage('Search GitHub for Mamba implementations')", cls="quick-btn"),
                        Button("Knowledge base", onclick="sendMessage('What topics do I have in my knowledge base?')", cls="quick-btn"),
                        cls="quick-actions",
                    ),

                    Div(
                        Form(
                            Input(
                                type="text",
                                name="message",
                                placeholder="Ask me to research any topic...",
                                autocomplete="off",
                                cls="chat-input",
                                id="message-input",
                            ),
                            Input(type="hidden", name="session_id", value=session_id),
                            Button("Send", type="submit", cls="send-btn"),
                            hx_post="/chat",
                            hx_target="#chat-messages",
                            hx_swap="beforeend",
                            hx_on_htmx_after_request="document.getElementById('message-input').value = ''; scrollToBottom();",
                            cls="chat-form",
                        ),
                        cls="chat-input-container",
                    ),

                    cls="chat-container",
                ),

                cls="container",
            ),

            Script("""
                function sendMessage(message) {
                    const input = document.getElementById('message-input');
                    input.value = message;
                    input.closest('form').dispatchEvent(new Event('submit', {bubbles: true}));
                }

                function scrollToBottom() {
                    const container = document.getElementById('chat-messages');
                    container.scrollTop = container.scrollHeight;
                }

                // Auto-scroll on new messages
                const observer = new MutationObserver(scrollToBottom);
                observer.observe(document.getElementById('chat-messages'), {
                    childList: true,
                    subtree: true
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
async def chat(request, message: str, session_id: str):
    """Handle chat messages."""
    logger.debug(f"Chat request: session={session_id}, message={repr(message)}")

    # Validate message
    if not message or not message.strip():
        logger.warning("Empty message received, ignoring")
        return Div()

    message = message.strip()
    session = get_session(session_id)

    # Add user message
    session["messages"].append({"role": "user", "content": message})

    # Send to agent
    logger.info(f"Sending to research agent: {message[:50]}...")
    response_text = await send_chat_message(
        RESEARCH_AGENT_URL,
        message,
        session["chat_id"],
    )

    # Add agent response
    session["messages"].append({"role": "agent", "content": response_text})

    # Return both messages
    return Div(
        render_message("user", message),
        render_message("agent", response_text),
    )


@rt("/clear", methods=["POST"])
async def clear_chat(session_id: str):
    """Clear chat history."""
    if session_id in sessions:
        sessions[session_id] = {
            "messages": [],
            "chat_id": str(uuid.uuid4().hex[:16]),
        }

    return RedirectResponse("/", status_code=303)


if __name__ == "__main__":
    import uvicorn

    logger.info(f"Starting Research Assistant Chat UI on port {WEB_PORT}")
    logger.info(f"Research Agent: {RESEARCH_AGENT_URL}")

    uvicorn.run(app, host="0.0.0.0", port=WEB_PORT)
