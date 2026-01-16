// Registry client for fetching registry from file or HTTP sources

use std::path::PathBuf;
use std::time::Duration;

use tracing::info;

use super::error::RegistryError;
use super::types::Registry;

/// Source for registry data
#[derive(Debug, Clone)]
pub enum RegistrySource {
	/// Load from local file
	File(PathBuf),
	/// Load from HTTP(S) URL
	Http {
		url: http::Uri,
		auth: Option<AuthConfig>,
	},
}

/// Authentication configuration for HTTP sources
#[derive(Debug, Clone)]
pub enum AuthConfig {
	/// Bearer token authentication
	Bearer(String),
	/// Basic authentication (username:password)
	Basic { username: String, password: String },
}

impl AuthConfig {
	/// Convert to HTTP Authorization header value
	pub fn to_header_value(&self) -> String {
		match self {
			AuthConfig::Bearer(token) => format!("Bearer {}", token),
			AuthConfig::Basic { username, password } => {
				let credentials = base64::Engine::encode(
					&base64::engine::general_purpose::STANDARD,
					format!("{}:{}", username, password),
				);
				format!("Basic {}", credentials)
			},
		}
	}
}

/// Client for fetching registry data
#[derive(Debug, Clone)]
pub struct RegistryClient {
	source: RegistrySource,
	refresh_interval: Duration,
}

impl RegistryClient {
	/// Create a new registry client
	pub fn new(source: RegistrySource, refresh_interval: Duration) -> Self {
		Self {
			source,
			refresh_interval,
		}
	}

	/// Create a registry client from a source URI string
	pub fn from_uri(uri: &str, refresh_interval: Duration, auth: Option<AuthConfig>) -> Result<Self, RegistryError> {
		let source = if uri.starts_with("file://") {
			let path = uri.strip_prefix("file://").unwrap();
			RegistrySource::File(PathBuf::from(path))
		} else if uri.starts_with("http://") || uri.starts_with("https://") {
			let url = uri
				.parse::<http::Uri>()
				.map_err(|e| RegistryError::InvalidSource(format!("invalid URL: {}", e)))?;
			RegistrySource::Http { url, auth }
		} else {
			return Err(RegistryError::InvalidSource(format!(
				"unsupported URI scheme: {}",
				uri
			)));
		};

		Ok(Self::new(source, refresh_interval))
	}

	/// Get the source configuration
	pub fn source(&self) -> &RegistrySource {
		&self.source
	}

	/// Get the refresh interval
	pub fn refresh_interval(&self) -> Duration {
		self.refresh_interval
	}

	/// Fetch the registry from the configured source
	pub async fn fetch(&self) -> Result<Registry, RegistryError> {
		match &self.source {
			RegistrySource::File(path) => self.fetch_from_file(path).await,
			RegistrySource::Http { url, auth } => self.fetch_from_http(url, auth.as_ref()).await,
		}
	}

	/// Fetch registry from a local file
	async fn fetch_from_file(&self, path: &PathBuf) -> Result<Registry, RegistryError> {
		info!(target: "virtual_tools", "Loading registry from file: {}", path.display());
		let content = fs_err::tokio::read_to_string(path).await?;
		let registry: Registry = serde_json::from_str(&content)?;
		info!(target: "virtual_tools", "Loaded {} tools from registry file", registry.len());
		Ok(registry)
	}

	/// Fetch registry from HTTP(S) URL
	#[cfg(feature = "testing")]
	async fn fetch_from_http(
		&self,
		url: &http::Uri,
		auth: Option<&AuthConfig>,
	) -> Result<Registry, RegistryError> {
		info!(target: "virtual_tools", "Fetching registry from HTTP: {}", url);

		// Build the request
		let client = reqwest::Client::new();
		let mut request = client.get(url.to_string());

		// Add authentication if configured
		if let Some(auth_config) = auth {
			request = request.header("Authorization", auth_config.to_header_value());
		}

		// Execute the request
		let response = request.send().await.map_err(|e| {
			RegistryError::FetchError(format!("HTTP request failed: {}", e))
		})?;

		// Check status
		if !response.status().is_success() {
			return Err(RegistryError::FetchError(format!(
				"HTTP request failed with status: {}",
				response.status()
			)));
		}

		// Parse response body
		let body = response.text().await.map_err(|e| {
			RegistryError::FetchError(format!("Failed to read response body: {}", e))
		})?;

		let registry: Registry = serde_json::from_str(&body)?;
		info!(target: "virtual_tools", "Fetched {} tools from registry URL", registry.len());
		Ok(registry)
	}

	/// Fetch registry from HTTP(S) URL (stub when testing feature is not enabled)
	#[cfg(not(feature = "testing"))]
	async fn fetch_from_http(
		&self,
		url: &http::Uri,
		_auth: Option<&AuthConfig>,
	) -> Result<Registry, RegistryError> {
		Err(RegistryError::FetchError(format!(
			"HTTP registry fetching requires the 'testing' feature: {}",
			url
		)))
	}

	/// Check if this is a file source (for file watching)
	pub fn is_file_source(&self) -> bool {
		matches!(self.source, RegistrySource::File(_))
	}

	/// Get the file path if this is a file source
	pub fn file_path(&self) -> Option<&PathBuf> {
		match &self.source {
			RegistrySource::File(path) => Some(path),
			_ => None,
		}
	}
}

/// Parse a duration string like "5m", "30s", "1h"
pub fn parse_duration(s: &str) -> Result<Duration, RegistryError> {
	let s = s.trim();
	if s.is_empty() {
		return Err(RegistryError::InvalidSource("empty duration string".into()));
	}

	let (num_str, unit) = if s.ends_with("ms") {
		(&s[..s.len() - 2], "ms")
	} else if s.ends_with('s') {
		(&s[..s.len() - 1], "s")
	} else if s.ends_with('m') {
		(&s[..s.len() - 1], "m")
	} else if s.ends_with('h') {
		(&s[..s.len() - 1], "h")
	} else if s.ends_with('d') {
		(&s[..s.len() - 1], "d")
	} else {
		// Assume seconds if no unit
		(s, "s")
	};

	let num: u64 = num_str
		.parse()
		.map_err(|_| RegistryError::InvalidSource(format!("invalid duration number: {}", num_str)))?;

	let duration = match unit {
		"ms" => Duration::from_millis(num),
		"s" => Duration::from_secs(num),
		"m" => Duration::from_secs(num * 60),
		"h" => Duration::from_secs(num * 60 * 60),
		"d" => Duration::from_secs(num * 60 * 60 * 24),
		_ => {
			return Err(RegistryError::InvalidSource(format!(
				"unknown duration unit: {}",
				unit
			)))
		},
	};

	Ok(duration)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_duration() {
		assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
		assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
		assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
		assert_eq!(parse_duration("2d").unwrap(), Duration::from_secs(172800));
		assert_eq!(parse_duration("100ms").unwrap(), Duration::from_millis(100));
		assert_eq!(parse_duration("30").unwrap(), Duration::from_secs(30));
	}

	#[test]
	fn test_parse_duration_errors() {
		assert!(parse_duration("").is_err());
		assert!(parse_duration("abc").is_err());
		assert!(parse_duration("-5s").is_err());
	}

	#[test]
	fn test_from_uri_file() {
		let client = RegistryClient::from_uri(
			"file:///path/to/registry.json",
			Duration::from_secs(300),
			None,
		)
		.unwrap();

		assert!(client.is_file_source());
		assert_eq!(
			client.file_path(),
			Some(&PathBuf::from("/path/to/registry.json"))
		);
	}

	#[test]
	fn test_from_uri_http() {
		let client = RegistryClient::from_uri(
			"https://example.com/registry.json",
			Duration::from_secs(300),
			None,
		)
		.unwrap();

		assert!(!client.is_file_source());
		assert_eq!(client.file_path(), None);
	}

	#[test]
	fn test_from_uri_invalid() {
		assert!(RegistryClient::from_uri("ftp://example.com/registry.json", Duration::from_secs(300), None).is_err());
	}

	#[test]
	fn test_auth_config_bearer() {
		let auth = AuthConfig::Bearer("my-token".to_string());
		assert_eq!(auth.to_header_value(), "Bearer my-token");
	}

	#[test]
	fn test_auth_config_basic() {
		let auth = AuthConfig::Basic {
			username: "user".to_string(),
			password: "pass".to_string(),
		};
		// base64("user:pass") = "dXNlcjpwYXNz"
		assert_eq!(auth.to_header_value(), "Basic dXNlcjpwYXNz");
	}
}
