---
component: agentgateway-llm/policy
type: component
parent_feature: agentgateway-llm
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/llm/policy/mod.rs
  - crates/agentgateway/src/llm/policy/webhook.rs
  - crates/agentgateway/src/llm/policy/moderation.rs
  - crates/agentgateway/src/llm/policy/pii/mod.rs
  - crates/agentgateway/src/llm/policy/pii/pattern_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/email_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/phone_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/credit_card_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/us_ssn_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/ca_sin_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/url_recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/recognizer.rs
  - crates/agentgateway/src/llm/policy/pii/recognizer_result.rs
  - crates/agentgateway/src/llm/policy/tests.rs
  - crates/agentgateway/src/llm/policy/pii/tests.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# LLM Policy Engine

The `Policy` struct is the configuration surface for controlling LLM request/response behavior. It is applied in the request flow after body parsing and before forwarding to the upstream provider, and again on the response flow before returning to the client.

## Policy Capabilities

### 1. Defaults and Overrides

JSON-level field manipulation on the raw request body:

- **`defaults`**: Set field values only if not already present in the request (e.g. default temperature)
- **`overrides`**: Force field values regardless of what the client sent (e.g. enforce a model)

Applied during `unmarshal_request` via a slow path that round-trips through `serde_json::Value`.

### 2. Model Aliases

Map client-requested model names to actual provider model names:

- **Exact match**: `"gpt-4" -> "gpt-4-turbo-preview"` (O(1) HashMap lookup)
- **Wildcard patterns**: `"claude-*" -> "claude-3-opus"` (compiled to regex, sorted by specificity -- longer patterns match first)

Resolved via `resolve_model_alias()` which tries exact match first, then falls back to pattern matching.

### 3. Prompt Enrichment

Inject additional messages into the conversation:

- **`prepend`**: Messages added before the client's messages (e.g. system instructions)
- **`append`**: Messages added after the client's messages (e.g. output format instructions)

Uses `SimpleChatCompletionMessage` (role + content) and calls `RequestType::prepend_prompts` / `append_prompts`.

### 4. Prompt Guards

Bidirectional content inspection with three enforcement mechanisms:

#### Request Guards (`PromptGuard.request`)

Applied to client messages before they reach the LLM:

| Guard Type | Module | Behavior |
|-----------|--------|----------|
| **Regex** | `policy/mod.rs` | Pattern matching with builtin PII recognizers or custom regex. Actions: reject or mask. |
| **Webhook** | `policy/webhook.rs` | External guardrail service called with message content. Can pass, mask, or reject with custom status/body. |
| **OpenAI Moderation** | `policy/moderation.rs` | Calls OpenAI's moderation API endpoint. Rejects if any result is flagged. |

#### Response Guards (`PromptGuard.response`)

Applied to LLM responses before they reach the client. Same guard types: Regex and Webhook (no moderation on responses).

#### Guardrail Outcomes

Every guard evaluation results in one of three outcomes, each recorded as a metric:

- **Allow** -- Content passes, no modification
- **Mask** -- Content is modified (PII redacted, webhook-rewritten) but still forwarded
- **Reject** -- Request/response is blocked, error response returned to client

### 5. Prompt Caching

`PromptCachingConfig` controls Anthropic-style prompt caching behavior:

- `cache_system` -- Cache system messages (default: true)
- `cache_messages` -- Cache conversation messages (default: true)
- `cache_tools` -- Cache tool definitions (default: false)
- `min_tokens` -- Minimum token threshold to activate caching (default: 1024)

Applied during Bedrock request serialization to inject `cache_point` markers.

### 6. Route Resolution

`SortedRoutes` maps URL path suffixes to `RouteType` values, determining how a request is parsed. Routes are stored longest-first for specificity, with `"*"` (wildcard) always last. Default route type is `Completions`.

## PII Detection (`policy/pii/`)

Built-in regex-based recognizers for sensitive data patterns:

| Recognizer | Detects |
|-----------|---------|
| `EmailRecognizer` | Email addresses |
| `PhoneRecognizer` | Phone numbers |
| `CreditCardRecognizer` | Credit card numbers (with Luhn validation) |
| `UsSsnRecognizer` | US Social Security Numbers |
| `CaSinRecognizer` | Canadian Social Insurance Numbers |

All implement the `Recognizer` trait and are initialized as lazy statics. Used by the `Builtin` regex guard variant.

## Webhook Protocol

The webhook guardrail (`policy/webhook.rs`) communicates with external services using a structured JSON protocol:

- **Request path**: Sends `GuardrailsPromptRequest` with message array
- **Response path**: Sends `GuardrailsResponseRequest` with choice array
- External service responds with an action: `Pass`, `Mask` (with rewritten content), or `Reject` (with status code and body)
- Supports forwarding specific request headers to the webhook via `forward_header_matches`

## Related

- [[features/agentgateway-llm|LLM Request Passthrough]] -- Parent feature
- [[components/agentgateway-llm/types|Types]] -- `RequestType` and `ResponseType` traits used for uniform policy application
- [[components/agentgateway-llm/conversion|Conversion]] -- Runs after policy in the request flow
- [[features/core|Core]] -- Metrics infrastructure for guardrail trip recording
