---
type: concept
related_features:
  - agentgateway-proxy
  - agentgateway-mcp
  - agentgateway-a2a
  - agentgateway-llm
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/concept
  - doctrack/status/active
  - doctrack/audience/claude
---

# Protocol Multiplexing

## What It Is
Agentgateway serves as a unified data plane for multiple agentic AI protocols. A single gateway instance can handle MCP (tool calls), A2A (agent-to-agent), LLM passthrough (OpenAI/Anthropic API), and plain HTTP — all on the same port, differentiated by routing rules.

## Protocols Supported

| Protocol | Module | Transport |
|----------|--------|-----------|
| MCP | [[features/agentgateway-mcp\|MCP]] | SSE, StreamableHTTP |
| A2A | [[features/agentgateway-a2a\|A2A]] | HTTP/JSON-RPC |
| LLM | [[features/agentgateway-llm\|LLM]] | HTTP (passthrough) |
| HTTP | [[features/agentgateway-http\|HTTP]] | HTTP/1.1, HTTP/2 |
| TCP | [[features/agentgateway-proxy\|Proxy]] | Raw TCP |

## Where It Appears
- [[features/agentgateway-proxy|Proxy]] — route dispatch based on listener protocol
- Configuration: `listeners[].protocol` selects MCP, A2A, LLM, or HTTP handling
- [[features/agentgateway-transport|Transport]] — connection-level protocol detection
