// Registry error types

use thiserror::Error;

/// Errors that can occur during registry operations
#[derive(Error, Debug)]
pub enum RegistryError {
	#[error("failed to parse registry: {0}")]
	ParseError(#[from] serde_json::Error),

	#[error("failed to read registry file: {0}")]
	IoError(#[from] std::io::Error),

	#[error("failed to fetch registry: {0}")]
	FetchError(String),

	#[error("invalid JSONPath expression '{path}': {message}")]
	InvalidJsonPath { path: String, message: String },

	#[error("JSONPath evaluation failed for '{path}': {message}")]
	JsonPathEvaluation { path: String, message: String },

	#[error("virtual tool '{name}' not found in registry")]
	ToolNotFound { name: String },

	#[error("source tool '{target}/{tool}' not found for virtual tool '{virtual_name}'")]
	SourceToolNotFound {
		virtual_name: String,
		target: String,
		tool: String,
	},

	#[error("environment variable '{name}' not found")]
	EnvVarNotFound { name: String },

	#[error("invalid registry source URI: {0}")]
	InvalidSource(String),

	#[error("schema validation error: {0}")]
	SchemaValidation(String),

	#[error("compilation error: {0}")]
	CompilationError(String),

	#[error("duplicate tool name: '{0}'")]
	DuplicateToolName(String),

	#[error("reference depth exceeded for tool '{0}' (possible circular reference)")]
	ReferenceDepthExceeded(String),

	#[error("tool '{0}' is a composition and requires the executor (cannot use prepare_call_args)")]
	CompositionRequiresExecutor(String),

	#[error("unknown tool reference: '{0}'")]
	UnknownToolReference(String),
}

impl RegistryError {
	pub fn invalid_jsonpath(path: impl Into<String>, message: impl Into<String>) -> Self {
		Self::InvalidJsonPath {
			path: path.into(),
			message: message.into(),
		}
	}

	pub fn jsonpath_eval(path: impl Into<String>, message: impl Into<String>) -> Self {
		Self::JsonPathEvaluation {
			path: path.into(),
			message: message.into(),
		}
	}

	pub fn tool_not_found(name: impl Into<String>) -> Self {
		Self::ToolNotFound { name: name.into() }
	}

	pub fn source_not_found(
		virtual_name: impl Into<String>,
		target: impl Into<String>,
		tool: impl Into<String>,
	) -> Self {
		Self::SourceToolNotFound {
			virtual_name: virtual_name.into(),
			target: target.into(),
			tool: tool.into(),
		}
	}
}
