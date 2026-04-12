---
feature: agentgateway-http
type: feature
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/http/mod.rs
  - crates/agentgateway/src/http/authorization.rs
  - crates/agentgateway/src/http/jwt.rs
  - crates/agentgateway/src/http/auth.rs
  - crates/agentgateway/src/http/apikey.rs
  - crates/agentgateway/src/http/basicauth.rs
  - crates/agentgateway/src/http/sessionpersistence.rs
  - crates/agentgateway/src/http/filters.rs
  - crates/agentgateway/src/http/cors.rs
  - crates/agentgateway/src/http/localratelimit.rs
  - crates/agentgateway/src/http/remoteratelimit.rs
  - crates/agentgateway/src/http/retry/mod.rs
  - crates/agentgateway/src/http/deadletter/mod.rs
  - crates/agentgateway/src/http/compression/mod.rs
  - crates/agentgateway/src/http/enricher.rs
  - crates/agentgateway/src/http/ext_proc.rs
  - crates/agentgateway/src/http/ext_authz.rs
  - crates/agentgateway/src/http/wiretap.rs
  - crates/agentgateway/src/http/stateful.rs
  - crates/agentgateway/src/http/transformation_cel.rs
  - crates/agentgateway/src/http/route.rs
  - crates/agentgateway/src/http/timeout.rs
  - crates/agentgateway/src/http/csrf.rs
  - crates/agentgateway/src/http/idempotent.rs
  - crates/agentgateway/src/http/outlierdetection.rs
  - crates/agentgateway/src/http/backendtls.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/feature
  - doctrack/status/active
  - doctrack/audience/claude
---

# HTTP Module

The HTTP module (`crates/agentgateway/src/http/`) is the foundational layer for all request/response handling in agentgateway. It provides the core HTTP types, a comprehensive authentication/authorization stack, traffic management policies, and integration patterns that the [[features/agentgateway-proxy|Proxy]] and [[features/agentgateway-mcp|MCP]] layers build upon.

## Architecture

```mermaid
graph TD
  subgraph "Inbound Authentication"
    JWT["JWT (JWKS)"]
    AK["API Key"]
    BA["Basic Auth"]
    EA["Ext AuthZ"]
  end

  subgraph "Policy Evaluation"
    AZ["Authorization (CEL)"]
    CORS["CORS"]
    CSRF["CSRF"]
    RL["Rate Limiting"]
    TO["Timeout"]
  end

  subgraph "Request Processing"
    FI["Filters (header/redirect/rewrite)"]
    TC["Transformation (CEL)"]
    EN["Enricher"]
    EP["Ext Proc"]
    CO["Compression"]
  end

  subgraph "Backend Auth"
    AWS["AWS SigV4"]
    GCP["GCP Token"]
    AZR["Azure Token"]
    PT["Passthrough / Key"]
  end

  subgraph "Resilience"
    RT["Retry"]
    CB["Circuit Breaker"]
    DL["Dead Letter"]
    OD["Outlier Detection"]
    ID["Idempotent"]
  end

  subgraph "Observability"
    WT["WireTap"]
    SP["Session Persistence"]
  end

  JWT --> AZ
  AK --> AZ
  BA --> AZ
  EA --> AZ
  AZ --> FI
  FI --> EN
  EN --> EP
  EP --> CO
  RL --> RT
  RT --> DL

  class JWT,AK,BA,EA internal-link;
  class AZ,CORS,RL internal-link;
  class FI,TC,EN,EP,CO internal-link;
  class RT,CB,DL internal-link;
  class WT,SP internal-link;
```

## Core Types

Defined in `mod.rs`:

- **`Request`** / **`Response`** -- type aliases for `http::Request<Body>` / `http::Response<Body>` using `axum_core::body::Body`
- **`HeaderOrPseudo`** -- enum unifying regular HTTP headers with HTTP/2 pseudo-headers (`:method`, `:scheme`, `:authority`, `:path`, `:status`), used throughout for header manipulation
- **`PolicyResponse`** -- carries an optional direct response (short-circuit) plus optional response headers to merge, enabling filters to either modify headers or terminate the request early
- **`RequestOrResponse`** -- mutable enum for shared code that mutates either a request or response
- **`DropBody`** -- wrapper that ties resource lifetime to body consumption

Utility functions: `get_host`, `buffer_limit`, `read_body`, `inspect_body` (peek without consuming), `modify_req_uri`, `classify_content_type` (JSON vs SSE vs unknown).

Rate limit headers are defined in the `x_headers` submodule (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, etc.).

## Components

| Component | Note | Purpose |
|-----------|------|---------|
| Authentication | [[components/agentgateway-http/authentication]] | JWT, API Key, Basic Auth, backend auth (AWS/GCP/Azure) |
| Authorization & Policy | [[components/agentgateway-http/authorization-policy]] | CEL-based authorization rules, CORS, CSRF, rate limiting |
| Traffic Management | [[components/agentgateway-http/traffic-management]] | Retry, circuit breaker, dead letter, outlier detection, session persistence |
| Request Processing | [[components/agentgateway-http/request-processing]] | Filters, CEL transforms, enricher, ext_proc, compression, wiretap |

## Key Design Patterns

1. **CEL everywhere** -- Authorization rules, ext_authz metadata, transformation expressions, rate limit descriptors, and enricher inputs all use CEL expressions compiled at config load time. The `cel::ContextBuilder` registers expressions for lazy evaluation at request time.

2. **Three-mode authentication** -- JWT, API Key, and Basic Auth all support `Strict` (must be present), `Optional` (validate if present), and `Permissive`/default modes. Claims are stripped from the `Authorization` header and inserted into request extensions for downstream access.

3. **PolicyResponse merging** -- Filters return `PolicyResponse` values that either short-circuit (direct response) or accumulate response headers. The proxy merges these across the filter chain.

4. **Envoy-compatible extension points** -- `ext_proc` and `ext_authz` use Envoy's gRPC protobuf contracts (`envoy.service.ext_proc.v3`, `envoy.service.auth.v3`), enabling drop-in compatibility with existing Envoy extension servers.

5. **Enterprise Integration Patterns** -- The module implements several EIPs: WireTap (fire-and-forget side-channel copies), Enricher (parallel data augmentation), Dead Letter (failed request capture), and Idempotent Consumer.

## Related Features

- [[features/agentgateway-proxy|Proxy]] -- consumes HTTP types and policy responses
- [[features/agentgateway-mcp|MCP]] -- MCP session handling builds on session persistence
- [[features/core|Core]] -- telemetry, metrics, tracing infrastructure
