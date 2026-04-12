---
component: agentgateway-llm/conversion
type: component
parent_feature: agentgateway-llm
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/llm/conversion/mod.rs
  - crates/agentgateway/src/llm/conversion/messages.rs
  - crates/agentgateway/src/llm/conversion/completions.rs
  - crates/agentgateway/src/llm/conversion/responses.rs
  - crates/agentgateway/src/llm/conversion/bedrock.rs
  - crates/agentgateway/src/llm/conversion/bedrock_tests.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# LLM Format Conversion

Cross-provider request and response translation. When a client sends a request in one API format (e.g. OpenAI completions) but the backend provider expects a different format (e.g. Anthropic messages or Bedrock converse), this module performs the translation.

## Conversion Matrix

The gateway supports these input-to-provider translations:

| Input Format | Target Provider | Module | Notes |
|-------------|----------------|--------|-------|
| Completions | Anthropic | `messages::from_completions` | OpenAI completions to Anthropic messages |
| Completions | Bedrock | `bedrock::from_completions` | OpenAI completions to Bedrock converse |
| Messages | OpenAI | `completions::from_messages` | Anthropic messages to OpenAI completions |
| Messages | Bedrock | `bedrock::from_messages` | Anthropic messages to Bedrock converse |
| Responses | Bedrock | `bedrock::from_responses` | OpenAI responses to Bedrock converse |

Conversions that are **not** needed (same format in and out) use passthrough -- the request/response is serialized and forwarded without structural transformation.

## Translation Mechanics

Each conversion submodule provides:

1. **`translate(req)`** -- Converts a passthrough request to the target provider's wire format (`Vec<u8>`). Internally upgrades the passthrough type to its fully-typed variant via `json::convert`, performs the structural translation, then serializes.

2. **`translate_response(bytes)`** -- Parses a provider response and returns a `Box<dyn ResponseType>` in the original client's expected format.

3. **`translate_stream(body, buffer_limit, log)`** -- Streaming variant that wraps the SSE body, translating each event from the provider's format to the client's expected format.

## Key Translation: Completions to Messages

`messages::from_completions` (`conversion/messages.rs`) handles the most common cross-provider case:

- System messages are extracted from the message array and merged into Anthropic's top-level `system` field
- Message roles are mapped (`system` filtered out, `assistant` kept, everything else becomes `user`)
- OpenAI `tools` (function-calling) are mapped to Anthropic's `tool` format (name, description, input_schema)
- `max_tokens` / `max_completion_tokens` mapped to Anthropic's `max_tokens`
- `stop` sequences mapped to `stop_sequences`
- Streaming responses translate Anthropic's event types (`content_block_delta`, `message_delta`, `message_stop`) back to OpenAI's `choices[].delta` format

## Key Translation: Completions to Bedrock

`bedrock::from_completions` handles AWS Bedrock's distinct converse API:

- Messages mapped to Bedrock's `{role, content[{text}]}` format
- System prompt extracted to Bedrock's top-level `system` field
- Tools mapped to Bedrock's `toolConfig.tools[].toolSpec` format
- Response translated from Bedrock's `{output.message, usage}` back to OpenAI completions
- Streaming translates Bedrock's `contentBlockDelta`, `messageStop`, `metadata` events

## Streaming

All streaming paths use `parse::sse::json_passthrough` to parse SSE events incrementally. The pattern is:

1. Parse each SSE `data:` line as JSON into the source provider's event type
2. Translate to the target provider's event structure
3. Re-serialize and emit as SSE
4. Capture telemetry (token counts, first-token timing, completion text) as a side effect

Non-streaming responses are simpler: buffer the full body, parse, translate, re-serialize.

## Related

- [[features/agentgateway-llm|LLM Request Passthrough]] -- Parent feature
- [[components/agentgateway-llm/types|Types]] -- Passthrough and typed definitions used by conversions
- [[components/agentgateway-llm/policy|Policy]] -- Applied before conversion in the request flow
