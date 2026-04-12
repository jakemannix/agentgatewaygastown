---
type: concept
related_features:
  - agentgateway-llm
  - agentgateway-config
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/concept
  - doctrack/status/active
  - doctrack/audience/claude
---

# Passthrough Parsing Pattern

## What It Is
A pattern for handling LLM API requests where the gateway only needs to inspect or modify a few fields but must forward the entire request verbatim. Unknown fields pass through without deserialization.

```rust
#[serde(flatten, default)]
pub rest: serde_json::Value
```

## Why It Matters
LLM APIs (OpenAI, Anthropic, AWS Bedrock, Google) are large and evolving. Defining the full schema would be fragile and constantly out of date. Instead, agentgateway defines only the fields it operates on (model, messages, tools) and passes everything else through as opaque JSON.

## Where It Appears
- [[features/agentgateway-llm|LLM passthrough]] — primary user of this pattern
- [[features/agentgateway-config|Config parsing]] — `parse/passthrough.rs`
- [[features/agentgateway-mcp|MCP]] — similar pattern for MCP message forwarding

## Trade-offs
- **Pro**: Forward-compatible with API changes, minimal maintenance burden
- **Con**: Can't validate unknown fields, harder to test edge cases
