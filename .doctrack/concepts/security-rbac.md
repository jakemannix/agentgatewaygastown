---
type: concept
related_features:
  - agentgateway-mcp
  - agentgateway-http
  - agentgateway-proxy
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/concept
  - doctrack/status/active
  - doctrack/audience/claude
---

# Security and RBAC

## What It Is
Agentgateway provides a layered security model with multiple authentication mechanisms and MCP-specific RBAC (Role-Based Access Control).

## Authentication Methods
- **JWT** — JSON Web Token validation with JWKS endpoint support
- **Basic Auth** — htpasswd-based basic authentication
- **API Key** — Header/query-based API key validation
- **mTLS** — Mutual TLS with client certificate validation
- **External Auth** — Delegate to external authorization service (ext_authz)

## MCP RBAC
The RBAC system is MCP-specific — it understands tool names, prompt names, and resource URIs:
- `McpAuthorization` — individual rule (allow/deny tool X for identity Y)
- `McpAuthorizationSet` — collection of rules applied to a backend
- `CallerIdentity` — extracted from MCP `clientInfo` during initialize

## Where It Appears
- [[features/agentgateway-mcp|MCP]] — `rbac.rs`, `identity.rs`
- [[features/agentgateway-http|HTTP]] — JWT, basic auth, API key, ext_authz
- [[features/agentgateway-proxy|Proxy]] — auth enforcement before routing
- [[concepts/cel-policy-evaluation|CEL Policy]] — CEL expressions for authorization rules
