---
feature: agentgateway-http
type: component
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/http/authorization.rs
  - crates/agentgateway/src/http/cors.rs
  - crates/agentgateway/src/http/csrf.rs
  - crates/agentgateway/src/http/localratelimit.rs
  - crates/agentgateway/src/http/remoteratelimit.rs
  - crates/agentgateway/src/http/timeout.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Authorization and Policy

This component covers the policy evaluation layer that runs after authentication: CEL-based authorization rules, CORS, CSRF protection, rate limiting (local and remote), and request timeouts.

## CEL Authorization (`authorization.rs`)

The authorization engine evaluates CEL (Common Expression Language) expressions against request context to make allow/deny decisions.

### Data Model

- **`RuleSets`** -- a list of `RuleSet`, each containing a `PolicySet`
- **`PolicySet`** -- two lists: `allow` rules and `deny` rules, each a compiled `cel::Expression`
- **`Policy`** -- tagged enum: `Allow(Expression)` or `Deny(Expression)`

### Evaluation Logic

`RuleSets::validate()` implements deny-first logic:

1. If no rule sets have any rules, **allow** (open by default)
2. Build the CEL executor lazily (only when rules exist)
3. If **any** deny rule matches, **deny**
4. If **any** allow rule matches, **allow**
5. Otherwise, **deny** (closed by default when rules exist)

This means deny rules always take precedence. The executor is lazily constructed via a closure to avoid overhead when no rules are configured.

### Config Format

Rules can be specified as plain strings (treated as allow) or objects with explicit `Allow`/`Deny` tags:

```yaml
authorization:
  - rules:
    - "jwt.sub == 'admin'"                    # plain string = allow
    - Allow: "jwt.groups.contains('editors')" # explicit allow
    - Deny: "jwt.iss == 'untrusted'"          # explicit deny
```

Expressions are compiled at config load time via `cel::Expression::new_strict`, failing fast on invalid CEL.

### CEL Context

Expressions can reference variables populated by upstream middleware:
- `jwt.*` -- JWT claims (from JWT authentication)
- `apikey.*` -- API key metadata
- `basicauth.*` -- Basic auth username
- `extauthz.*` -- Dynamic metadata from ext_authz
- `mcp.*` -- MCP-specific context (tool name, etc.)
- Standard request attributes: headers, method, path, source/destination

## CORS (`cors.rs`)

Follows Envoy semantics (`forwardNotMatchingPreflights=true`):

- `allow_origins`, `allow_methods`, `allow_headers` -- each can be a wildcard `["*"]`, an explicit list, or empty (none)
- Preflight (`OPTIONS`) requests get a direct `200 OK` response with appropriate headers
- Non-preflight requests with matching origin get CORS headers added to the response via `PolicyResponse.response_headers`
- `allow_credentials`, `expose_headers`, `max_age` supported
- Non-matching origins are passed through without CORS headers

## Rate Limiting

### Local Rate Limiting (`localratelimit.rs`)

Token-bucket rate limiter, forked from `pelikan-io/rustcommon`:

- Configured via `max_tokens`, `tokens_per_fill`, `fill_interval`
- **Request-based**: `check_request()` consumes 1 token per request
- **Token-based**: `check_llm_request()` consumes tokens proportional to LLM input token count. Uses `amend_tokens()` for post-hoc true-up after discovering actual response token usage.
- Lock-free implementation using `AtomicU64` and `compare_exchange` loops
- Returns `ProxyError::RateLimitExceeded` with limit, remaining, and reset info for response headers

### Remote Rate Limiting (`remoteratelimit.rs`)

Delegates rate limiting decisions to an external gRPC service using the Envoy rate limit protocol (`envoy.service.ratelimit.v3`):

- `domain` scopes the rate limit namespace
- `descriptors` are CEL expressions that generate key-value pairs sent to the service
- Supports both request-based and token-based limiting
- Configurable timeout for the remote call

## Timeout (`timeout.rs`)

Request timeout configuration applied as `BackendRequestTimeout` extension on the request.

## CSRF (`csrf.rs`)

Cross-Site Request Forgery protection.

## Related

- [[components/agentgateway-http/authentication]] -- populates claims used by CEL authorization
- [[features/agentgateway-http|HTTP Module]] -- parent feature
- [[features/agentgateway-proxy|Proxy]] -- applies policies during request processing
