---
feature: agentgateway-http
type: component
doctrack_version: 3.0.0
files:
  - crates/agentgateway/src/http/jwt.rs
  - crates/agentgateway/src/http/apikey.rs
  - crates/agentgateway/src/http/basicauth.rs
  - crates/agentgateway/src/http/auth.rs
  - crates/agentgateway/src/http/ext_authz.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Authentication

The authentication subsystem provides inbound credential verification (JWT, API Key, Basic Auth) and outbound backend authentication (AWS, GCP, Azure, passthrough, static key). It also supports delegated authorization via external services (ext_authz).

## Inbound Authentication

All three inbound mechanisms share a common pattern:

1. Extract credentials from the `Authorization` header (Bearer or Basic)
2. Validate against configured sources (JWKS, key list, htpasswd)
3. Strip the `Authorization` header from the request
4. Insert typed claims into request extensions for downstream use
5. Register claims with the CEL context for use in authorization rules

### JWT (`jwt.rs`)

- **`Jwt`** struct holds a `Mode` and a list of `Provider`s
- Each `Provider` loads a JWKS (from file, inline, or remote URL) and builds `DecodingKey` + `Validation` per key ID
- Supports RSA (RS256/384/512) and Elliptic Curve (ES256/384) algorithms
- Algorithm inference: if `key_algorithm` is not set in the JWK, algorithms are inferred from key type (RSA vs EC)
- Multi-provider support: keys are searched across all providers by `kid`
- **`Claims`** wraps `Map<String, Value>` (arbitrary JSON claims) plus the raw JWT as a `SecretString`
- `Mode::Strict` -- token required, `Mode::Optional` -- validate if present, `Mode::Permissive` -- never reject
- Config: `LocalJwtConfig` supports single-provider shorthand or multi-provider with per-provider audiences

### API Key (`apikey.rs`)

- **`APIKeyAuthentication`** holds a `HashMap<APIKey, UserMetadata>` mapping keys to arbitrary JSON metadata
- Keys are stored as `SecretString`, compared via exposed secret
- Extracted from Bearer token in the Authorization header
- `Mode::Strict` or `Mode::Optional`
- Config: `LocalAPIKeys` with a list of `{key, metadata}` entries

### Basic Auth (`basicauth.rs`)

- **`BasicAuthentication`** uses `htpasswd_verify::Htpasswd` for credential verification
- Supports standard htpasswd format (bcrypt, SHA-1, MD5, etc.)
- Configurable `realm` for WWW-Authenticate responses
- `Mode::Strict` or `Mode::Optional`

## External Authorization (`ext_authz.rs`)

Delegates auth decisions to an external service using Envoy's ext_authz protocol:

- **gRPC protocol** -- sends `CheckRequest` with `AttributeContext` (headers, source/destination info, metadata). Supports `context_extensions` and dynamic `metadata` via CEL expressions.
- **HTTP protocol** -- forwards request to an HTTP authorization endpoint. Supports `redirect` on unauthorized, `include_response_headers` to copy auth response headers into the proxied request, and `add_request_headers` via CEL.
- **Failure modes**: `Allow` (fail-open), `Deny` (fail-closed, default), `DenyWithStatus(u16)`
- Request body can be included with configurable `max_request_bytes` and `allow_partial_message`
- Dynamic metadata from ext_authz responses is stored in `ExtAuthzDynamicMetadata` for downstream CEL access

## Backend Authentication (`auth.rs`)

Applied when forwarding requests to upstream backends:

| Method | Type | How it works |
|--------|------|-------------|
| `Passthrough` | Re-attaches the original JWT | Reads `Claims` from request extensions, re-inserts Bearer token |
| `Key` | Static API key | Inserts configured key as Bearer token (from file or inline, stored as `SecretString`) |
| `Gcp` | Google Cloud | `IdToken` (audience-scoped, with user ADC or service account) or `AccessToken` via `google-cloud-auth` |
| `Aws` | AWS SigV4 | Request signing via `aws-sigv4` for Bedrock. Uses `apply_late_backend_auth` (must run after body is finalized). Supports explicit credentials or implicit (env/IAM). |
| `Azure` | Azure AD | `ClientSecret`, `ManagedIdentity`, `WorkloadIdentity`, or `DeveloperImplicit`. Fetches token via `azure-identity` SDK. |

GCP uses lazy-initialized static credentials (`Lazy<AccessTokenCredentials>`) and an audience-keyed ID token cache.

AWS signing is deferred to `apply_late_backend_auth` because SigV4 must sign the final request body.

## Related

- [[features/agentgateway-http|HTTP Module]] -- parent feature
- [[components/agentgateway-http/authorization-policy]] -- CEL authorization rules applied after authentication
