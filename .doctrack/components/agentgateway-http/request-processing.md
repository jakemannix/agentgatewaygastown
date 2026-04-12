---
feature: agentgateway-http
type: component
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/http/filters.rs
  - crates/agentgateway/src/http/transformation_cel.rs
  - crates/agentgateway/src/http/enricher.rs
  - crates/agentgateway/src/http/ext_proc.rs
  - crates/agentgateway/src/http/compression/mod.rs
  - crates/agentgateway/src/http/wiretap.rs
  - crates/agentgateway/src/http/route.rs
  - crates/agentgateway/src/http/backendtls.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Request Processing

This component covers the request/response transformation pipeline: HTTP filters, CEL-based transformations, the enricher pattern, external processing, compression, wire tapping, routing, and backend TLS.

## HTTP Filters (`filters.rs`)

Core request/response modification primitives:

### HeaderModifier
Adds, sets, or removes headers:
- `add` -- append headers (allows duplicates)
- `set` -- insert/replace headers
- `remove` -- delete headers by name

### RequestRedirect
Returns a redirect response (`302 Found` by default):
- `scheme` -- override scheme (http/https)
- `authority` -- `HostRedirect` variants: `Full`, `Host`, `Port`, `Auto`, `None`
- `path` -- `PathRedirect` variants: `Full` (complete path) or `Prefix` (rewrite prefix based on matched path)
- `status` -- custom redirect status code
- Preserves query parameters across redirects

### UrlRewrite
In-place URI rewriting (does not redirect):
- Same `authority` and `path` options as `RequestRedirect`
- Stores the `OriginalUrl` in request extensions for reference
- `AutoHostname` extension signals backend-based host rewriting

### DirectResponse
Returns a static response (body + status code) without forwarding to a backend.

### RequestMirror
Sends a copy of the request to a secondary backend with percentage-based sampling.

## CEL Transformations (`transformation_cel.rs`)

Header and body transformations using compiled CEL expressions:

- **`TransformerConfig`** -- separate request and response transforms
- Each transform supports `add`, `set`, `remove` (headers) and `body` (body replacement)
- Header names support `HeaderOrPseudo` -- can set HTTP/2 pseudo-headers like `:path`, `:authority`
- CEL expressions are compiled at config time (`new_strict` for validated, `new_permissive` for best-effort)
- `LocalTransformationConfig` is the deserialization format; converted to compiled `TransformerConfig` at load

## Enricher Pattern (`enricher.rs`)

Enterprise Integration Pattern for augmenting requests with parallel data lookups:

- **`EnricherSpec`** configures a list of `EnrichmentSource`s called in parallel
- Each `EnrichmentSource` has:
  - `field` -- result key name
  - `backend` -- service to call
  - `input` -- optional CEL expression to transform the input before calling
- **`MergeStrategy`** for combining results:
  - `Spread` -- merge all enrichment fields into root object
  - `Nested { key }` -- put results under a single key
  - `SchemaMap` -- map results according to a schema definition
- `ignore_failures` -- if true, failed enrichments are skipped; if false, any failure aborts
- `timeout_ms` -- optional timeout for enrichment calls

## External Processing (`ext_proc.rs`)

Delegates request/response processing to an external gRPC service using the Envoy ext_proc protocol (`envoy.service.ext_proc.v3`):

- Bidirectional gRPC streaming: sends `ProcessingRequest` messages (request headers, request body, response headers, response body, trailers), receives `ProcessingResponse` messages
- Responses can mutate headers (`HeaderMutation`), replace body (`BodyMutation`), or send an `ImmediateResponse` (short-circuit)
- **Failure modes**: `FailClosed` (default) or `FailOpen`
- Communicates via `mpsc::Sender`/`Receiver` channels for streaming
- ~27KB implementation covering the full request/response lifecycle

## Compression (`compression/mod.rs`)

Transparent content encoding/decoding:

- Supports gzip, deflate (zlib), Brotli (`br`), and Zstandard (`zstd`)
- `to_bytes_with_decompression()` -- decodes response bodies based on `Content-Encoding` header
- `encode_body()` -- compresses a byte slice with the specified encoding
- Uses `async-compression` crate with tokio `BufReader`/`StreamReader` adapters
- Respects buffer size limits to prevent OOM

## WireTap (`wiretap.rs`)

Enterprise Integration Pattern for side-channel message copies:

- **Fire-and-forget**: tap failures never affect the main request flow
- **`TapPoint`**: `Before`, `After`, or `Both` the main operation
- **`TapTarget`**: backend destination + percentage-based sampling (0.0-1.0)
- Multiple targets supported per WireTap
- Use cases: auditing, debugging, logging, compliance recording

## Routing (`route.rs`)

Route matching and selection logic. ~6KB covering path matching, header matching, and route priority resolution.

## Backend TLS (`backendtls.rs`)

TLS configuration for upstream backend connections. ~7KB covering certificate loading, verification, and mTLS.

## Related

- [[features/agentgateway-http|HTTP Module]] -- parent feature
- [[components/agentgateway-http/authorization-policy]] -- policies evaluated before request processing
- [[features/agentgateway-proxy|Proxy]] -- orchestrates the filter/transform pipeline
- [[features/core|Core]] -- telemetry hooks into enricher and ext_proc
