---
feature: a2a-sdk
type: feature
doctrack_version: 3.0.0
files:
  - crates/a2a-sdk/Cargo.toml
  - crates/a2a-sdk/src/lib.rs
  - crates/a2a-sdk/src/jsonrpc.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# A2A SDK

## Purpose

Standalone Rust crate providing serde-compatible type definitions for Google's **Agent2Agent (A2A) protocol** (v0.3.0). Published to crates.io as `a2a-sdk` (v0.7.0). This is a pure data types crate — no networking, no runtime, no async — just `Serialize`/`Deserialize` structs and enums that other crates (primarily [[features/agentgateway-a2a|agentgateway's A2A handling]]) depend on.

## Dependencies

Minimal — only three workspace dependencies:
- `chrono` — timestamp handling in `Message`
- `serde` / `serde_json` — serialization for all types

## Source Files

### `jsonrpc.rs` — JSON-RPC 2.0 Foundation

Low-level JSON-RPC plumbing, inspired by the `rmcp` crate:

| Type | Description |
|------|-------------|
| `JsonRpcVersion2_0` | Const-string type that serializes/deserializes only `"2.0"` |
| `NumberOrString` / `RequestId` | JSON-RPC id field (integer or string) |
| `JsonRpcRequest<R>` | Generic request envelope with `jsonrpc`, `id`, flattened request body |
| `JsonRpcResponse<R>` | Generic response envelope with `jsonrpc`, `id`, `result` |
| `Request<M, P>` | Method + params pair, generic over method const-string and params type |
| `JsonObject` | Type alias for `serde_json::Map<String, Value>` |
| `const_string!` macro | Generates zero-size types that serialize to/from a single fixed string literal |

### `lib.rs` — A2A Protocol Types

All A2A domain types, organized into these categories:

**Top-level message envelope:**
- `JsonRpcMessage` — untagged enum: `Request(A2aRequest)` | `Response(A2aResponse)` | `Error(JsonRpcError)`
- `JsonRpcError`, `ErrorData`, `ErrorCode`

**Request types (A2aRequest enum):**

Legacy (deprecated, kept for backward compatibility):
- `SendTaskRequest` (`tasks/send`)
- `SendSubscribeTaskRequest` (`tasks/sendSubscribe`)
- `TaskPushNotificationGetRequest` / `SetRequest`
- `TaskResubscribeRequest`

Current (v0.3.0):
- `SendMessageRequest` (`message/send`)
- `SendStreamingMessageRequest` (`message/stream`)
- `GetTaskRequest` (`tasks/get`)
- `CancelTaskRequest` (`tasks/cancel`)
- `SetTaskPushNotificationConfigRequest` (`tasks/pushNotificationConfig/set`)
- `GetTaskPushNotificationConfigRequest` (`tasks/pushNotificationConfig/get`)
- `ListTaskPushNotificationConfigRequest` (`tasks/pushNotificationConfig/list`)
- `DeleteTaskPushNotificationConfigRequest` (`tasks/pushNotificationConfig/delete`)
- `TaskResubscriptionRequest` (`tasks/resubscribe`)
- `GetAuthenticatedExtendedCardRequest` (`agent/getAuthenticatedExtendedCard`)

**Response types (A2aResponse enum):**
- Legacy: `SendTaskResponse`, `SendTaskUpdateResponse`
- Current: `SendMessageResponse`, `SendStreamingMessageResponse`, `GetTaskResponse`, `CancelTaskResponse`, plus push notification config responses
- Each wraps a `*SuccessResponse` struct containing `id`, `jsonrpc`, and a typed `result`

**Core domain types:**

| Type | Description |
|------|-------------|
| `AgentCard` | Full agent descriptor: name, description, url, version, capabilities, skills, security schemes, protocol version, transport preferences |
| `AgentSkill` | Skill within an agent: id, name, description, examples, input/output modes, tags |
| `AgentCapabilities` | Feature flags: push notifications, state transition history, streaming, extensions |
| `Task` | A unit of work: id, context_id, status, artifacts, history, metadata |
| `TaskState` | Enum: Submitted, Working, InputRequired, Completed, Canceled, Failed, Unknown, Rejected, AuthRequired |
| `TaskStatus` | State + optional message + timestamp |
| `Message` | Content parts + role + timestamp + metadata |
| `Part` | Tagged enum: `Text(TextPart)` / `File(FilePart)` / `Data(DataPart)` |
| `Artifact` | Named collection of parts with artifact_id, description, metadata |
| `Role` | Enum: User / Agent |

**Security types:**
- `SecurityScheme` — tagged enum: ApiKey, HttpAuth, OAuth2, OpenIdConnect, MutualTLS
- `OAuthFlows` — authorization code, client credentials, implicit, password flows
- Full OAuth2 flow structs with token URLs, scopes, refresh URLs

**Parameter types:**
- `TaskSendParams`, `TaskIdParams`, `TaskQueryParams` — legacy task operations
- `MessageSendParams`, `MessageSendConfiguration` — current message operations
- `PushNotificationConfig`, `TaskPushNotificationConfig` — notification setup

**Error types:**
- `InternalError`, `InvalidParamsError`, `InvalidRequestError`, `MethodNotFoundError`
- `TaskNotFoundError`, `TaskNotCancelableError`, `PushNotificationNotSupportedError`, `UnsupportedOperationError`

**Streaming response types:**
- `SendTaskStreamingResponseResult` — untagged enum: Status update / Artifact update / None
- `TaskStatusUpdateEvent`, `TaskArtifactUpdateEvent`
- `SendStreamingMessageSuccessResponseResult` — Task / Message / StatusUpdate / ArtifactUpdate

## API Surface

This crate is **types-only**. The public API is:
1. Import types and deserialize incoming JSON-RPC messages via `serde_json::from_value::<JsonRpcMessage>()`
2. Construct request/response types and serialize them via `serde_json::to_value()`
3. Use `A2aRequest::method()` to get the JSON-RPC method string for routing
4. Use `A2aResponse::id()` to extract the task/message ID from any response variant

## Notes

- The crate carries both legacy and current type definitions side by side, with legacy fields marked `Option` and `skip_serializing_if` for backward compatibility
- Uses the `const_string!` macro pattern extensively — each JSON-RPC method name is a zero-size type that enforces correct serialization at compile time
- `#[serde(untagged)]` is used for `A2aRequest`, `A2aResponse`, and `JsonRpcMessage` — deserialization tries each variant in order
- `Part` uses `#[serde(tag = "kind")]` — internally tagged enum discriminated by the `kind` field
- `SecurityScheme` uses `#[serde(tag = "type")]` — internally tagged by `type` field
- The `error` module defines a simple `ConversionError` for `FromStr` / `TryFrom` conversions on enums
- One test (`test_serde`) validates round-trip deserialization of a completed task response

## Cross-References

- Consumed by [[features/agentgateway-a2a|agentgateway's A2A handling]] in `crates/agentgateway/src/a2a/`
- The `AgentCard` type is the A2A equivalent of an MCP server's capability advertisement
- `SecurityScheme` types mirror OpenAPI security scheme definitions
