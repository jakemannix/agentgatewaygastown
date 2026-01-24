// Caller Identity Module
//
// Extracts agent identity from incoming requests for dependency-scoped
// tool discovery and policy enforcement.
//
// Identity can be extracted from:
// 1. HTTP headers: X-Agent-Name, X-Agent-Version
// 2. JWT claims: agent_name, agent_version
// 3. MCP clientInfo (during initialize)

use http::HeaderMap;

/// Header names for agent identity
pub const AGENT_NAME_HEADER: &str = "x-agent-name";
pub const AGENT_VERSION_HEADER: &str = "x-agent-version";

/// JWT claim names for agent identity
pub const AGENT_NAME_CLAIM: &str = "agent_name";
pub const AGENT_VERSION_CLAIM: &str = "agent_version";

/// Caller identity extracted from request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerIdentity {
	/// Agent name (required)
	pub name: String,
	/// Agent version (optional)
	pub version: Option<String>,
	/// Source of the identity (for logging/debugging)
	pub source: IdentitySource,
}

/// Where the identity was extracted from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
	/// From X-Agent-Name/X-Agent-Version headers
	Headers,
	/// From JWT claims
	JwtClaims,
	/// From MCP initialize clientInfo
	McpClientInfo,
}

impl CallerIdentity {
	/// Create a new caller identity
	pub fn new(name: impl Into<String>, version: Option<String>, source: IdentitySource) -> Self {
		Self {
			name: name.into(),
			version,
			source,
		}
	}

	/// Extract identity from HTTP headers
	pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
		let name = headers
			.get(AGENT_NAME_HEADER)
			.and_then(|v| v.to_str().ok())
			.map(|s| s.to_string())?;

		let version = headers
			.get(AGENT_VERSION_HEADER)
			.and_then(|v| v.to_str().ok())
			.map(|s| s.to_string());

		Some(Self {
			name,
			version,
			source: IdentitySource::Headers,
		})
	}

	/// Extract identity from JWT claims
	pub fn from_claims(claims: &serde_json::Value) -> Option<Self> {
		let name = claims.get(AGENT_NAME_CLAIM)?.as_str()?.to_string();

		let version = claims
			.get(AGENT_VERSION_CLAIM)
			.and_then(|v| v.as_str())
			.map(|s| s.to_string());

		Some(Self {
			name,
			version,
			source: IdentitySource::JwtClaims,
		})
	}

	/// Extract identity from MCP clientInfo
	///
	/// The MCP InitializeRequestParam has a client_info field (Implementation) with name/version
	pub fn from_mcp_client_info(init_params: &rmcp::model::InitializeRequestParam) -> Option<Self> {
		let name = init_params.client_info.name.to_string();
		let version = Some(init_params.client_info.version.to_string());

		Some(Self {
			name,
			version,
			source: IdentitySource::McpClientInfo,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use http::header::HeaderValue;

	#[test]
	fn test_identity_from_headers() {
		let mut headers = HeaderMap::new();
		headers.insert(AGENT_NAME_HEADER, HeaderValue::from_static("customer-agent"));
		headers.insert(AGENT_VERSION_HEADER, HeaderValue::from_static("1.0.0"));

		let identity = CallerIdentity::from_headers(&headers).unwrap();
		assert_eq!(identity.name, "customer-agent");
		assert_eq!(identity.version, Some("1.0.0".to_string()));
		assert_eq!(identity.source, IdentitySource::Headers);
	}

	#[test]
	fn test_identity_from_headers_name_only() {
		let mut headers = HeaderMap::new();
		headers.insert(AGENT_NAME_HEADER, HeaderValue::from_static("test-agent"));

		let identity = CallerIdentity::from_headers(&headers).unwrap();
		assert_eq!(identity.name, "test-agent");
		assert_eq!(identity.version, None);
	}

	#[test]
	fn test_identity_from_headers_missing() {
		let headers = HeaderMap::new();
		assert!(CallerIdentity::from_headers(&headers).is_none());
	}

	#[test]
	fn test_identity_from_claims() {
		let claims = serde_json::json!({
			"sub": "user123",
			"agent_name": "research-agent",
			"agent_version": "2.1.0"
		});

		let identity = CallerIdentity::from_claims(&claims).unwrap();
		assert_eq!(identity.name, "research-agent");
		assert_eq!(identity.version, Some("2.1.0".to_string()));
		assert_eq!(identity.source, IdentitySource::JwtClaims);
	}

	#[test]
	fn test_identity_from_claims_missing() {
		let claims = serde_json::json!({
			"sub": "user123"
		});
		assert!(CallerIdentity::from_claims(&claims).is_none());
	}

	#[test]
	fn test_identity_from_mcp_client_info() {
		let init_params = rmcp::model::InitializeRequestParam {
			protocol_version: Default::default(),
			capabilities: Default::default(),
			client_info: rmcp::model::Implementation {
				name: "my-agent".to_string(),
				version: "1.0.0".to_string(),
				title: None,
				website_url: None,
				icons: None,
			},
		};

		let identity = CallerIdentity::from_mcp_client_info(&init_params).unwrap();
		assert_eq!(identity.name, "my-agent");
		assert_eq!(identity.version, Some("1.0.0".to_string()));
		assert_eq!(identity.source, IdentitySource::McpClientInfo);
	}
}
