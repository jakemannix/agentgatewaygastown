---
component: agentgateway-llm/types
type: component
parent_feature: agentgateway-llm
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/llm/types/mod.rs
  - crates/agentgateway/src/llm/types/completions.rs
  - crates/agentgateway/src/llm/types/messages.rs
  - crates/agentgateway/src/llm/types/responses.rs
  - crates/agentgateway/src/llm/types/embeddings.rs
  - crates/agentgateway/src/llm/types/count_tokens.rs
  - crates/agentgateway/src/llm/types/bedrock.rs
  - crates/agentgateway/src/llm/universal.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# LLM Types

Passthrough and typed request/response definitions for each LLM provider API format. Lives under `crates/agentgateway/src/llm/types/` plus the `universal.rs` file which re-exports `async_openai` types for use in the fully-typed conversion path.

## Design: Two-Layer Type System

Each API format has two representations:

1. **Passthrough types** (in `types/*.rs`) -- Define only fields the gateway operates on (model, messages, stream, temperature, etc.). Unknown fields are captured in `#[serde(flatten)] pub rest: serde_json::Value` and round-trip through serialization untouched.

2. **Typed types** (in `universal.rs` and `types/*/typed` submodules) -- Full type definitions used when cross-provider conversion is needed. The passthrough type is "upgraded" to the typed variant internally via `json::convert`.

## Trait Abstractions

`types/mod.rs` defines two key traits that all format-specific types implement:

### `RequestType`

```rust
pub trait RequestType: Send + Sync {
    fn model(&mut self) -> &mut Option<String>;
    fn prepend_prompts(&mut self, prompts: Vec<SimpleChatCompletionMessage>);
    fn append_prompts(&mut self, prompts: Vec<SimpleChatCompletionMessage>);
    fn to_llm_request(&self, provider: Strng, tokenize: bool) -> Result<LLMRequest, AIError>;
    fn get_messages(&self) -> Vec<SimpleChatCompletionMessage>;
    fn set_messages(&mut self, messages: Vec<SimpleChatCompletionMessage>);
    fn to_openai(&self) -> Result<Vec<u8>, AIError>;
    fn to_anthropic(&self) -> Result<Vec<u8>, AIError>;
    fn to_bedrock(&self, ...) -> Result<Vec<u8>, AIError>;
}
```

Enables the [[components/agentgateway-llm/policy|Policy]] layer to manipulate requests uniformly regardless of input format, and the [[components/agentgateway-llm/conversion|Conversion]] layer to serialize into the target provider's format.

### `ResponseType`

```rust
pub trait ResponseType: Send + Sync {
    fn to_llm_response(&self, include_completion_in_log: bool) -> LLMResponse;
    fn to_webhook_choices(&self) -> Vec<ResponseChoice>;
    fn set_webhook_choices(&mut self, resp: Vec<ResponseChoice>) -> anyhow::Result<()>;
    fn serialize(&self) -> serde_json::Result<Vec<u8>>;
}
```

Enables uniform telemetry extraction and response-side prompt guard evaluation.

### `SimpleChatCompletionMessage`

A minimal `{role, content}` struct that serves as the lingua franca for policy operations (prompt enrichment, guards, webhooks). All format-specific types convert to/from this for policy interactions.

## Format Modules

| Module | Input Format | Key Fields Parsed | Passthrough Fields |
|--------|-------------|-------------------|-------------------|
| `completions.rs` | OpenAI chat completions | model, messages, stream, temperature, top_p, frequency_penalty, presence_penalty, seed, max_tokens, stream_options | tools, response_format, logit_bias, reasoning_effort, etc. |
| `messages.rs` | Anthropic messages | model, messages, system, stream, temperature, top_p, max_tokens | tools, tool_choice, metadata, thinking, etc. |
| `responses.rs` | OpenAI responses | model, input, stream, temperature, top_p, max_output_tokens | tools, reasoning, instructions, previous_response_id, etc. |
| `embeddings.rs` | OpenAI embeddings | model, encoding_format, dimensions | input, user, etc. |
| `count_tokens.rs` | Anthropic token counting | model, messages, system | tools, etc. |
| `bedrock.rs` | AWS Bedrock converse | Bedrock-specific converse format | guardrail config, etc. |

## The `universal.rs` File

Re-exports the full `async_openai::types::chat` module (40+ types) plus defines a parallel set of passthrough-compatible traits (`RequestType`, `ResponseType`). This is used by the typed conversion path and by `universal.rs`'s own passthrough type definitions that wrap `async_openai` types with the `rest` catch-all field.

## Related

- [[features/agentgateway-llm|LLM Request Passthrough]] -- Parent feature
- [[components/agentgateway-llm/conversion|Conversion]] -- Uses typed variants for cross-format translation
- [[components/agentgateway-llm/policy|Policy]] -- Operates on requests/responses via trait abstractions
