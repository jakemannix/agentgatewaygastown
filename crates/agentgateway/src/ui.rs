use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, Method};
use hyper::body::Incoming;
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use tower::ServiceExt;
use tower_http::cors::CorsLayer;
use tower_serve_static::ServeDir;

use crate::management::admin::{AdminFallback, AdminResponse};
use crate::{Config, ConfigSource, client, yamlviajson};
pub struct UiHandler {
	router: Router,
}

#[derive(Clone, Debug)]
struct App {
	state: Arc<Config>,
	client: client::Client,
}

impl App {
	pub fn cfg(&self) -> Result<ConfigSource, ErrorResponse> {
		self
			.state
			.xds
			.local_config
			.clone()
			.ok_or(ErrorResponse::String("local config not setup".to_string()))
	}
}

lazy_static::lazy_static! {
	static ref ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../ui/out");
}

impl UiHandler {
	pub fn new(cfg: Arc<Config>) -> Self {
		let ui_service = ServeDir::new(&ASSETS_DIR);
		let router = Router::new()
			// Redirect to the UI
			.route("/config", get(get_config).post(write_config))
			.route("/registry", get(get_registry).post(write_registry))
			.nest_service("/ui", ui_service)
			.route("/", get(|| async { Redirect::permanent("/ui") }))
			.layer(add_cors_layer())
			.with_state(App {
				state: cfg.clone(),
				client: client::Client::new(&cfg.dns, None, Default::default(), None),
			});
		Self { router }
	}
}

#[derive(Debug, thiserror::Error)]
enum ErrorResponse {
	#[error("{0}")]
	String(String),
	#[error("{0}")]
	Anyhow(#[from] anyhow::Error),
}

impl Serialize for ErrorResponse {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl IntoResponse for ErrorResponse {
	fn into_response(self) -> Response {
		(StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
	}
}

async fn get_config(State(app): State<App>) -> Result<Json<Value>, ErrorResponse> {
	let s = app.cfg()?.read_to_string().await?;
	let v: Value = yamlviajson::from_str(&s).map_err(|e| ErrorResponse::Anyhow(e.into()))?;
	Ok(Json(v))
}

async fn write_config(
	State(app): State<App>,
	Json(config_json): Json<Value>,
) -> Result<Json<Value>, ErrorResponse> {
	let config_source = app.cfg()?;

	let file_path = match &config_source {
		ConfigSource::File(path) => path,
		ConfigSource::Static(_) => {
			return Err(ErrorResponse::String(
				"Cannot write to static config".to_string(),
			));
		},
	};
	let yaml_content =
		yamlviajson::to_string(&config_json).map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	if let Err(e) = crate::types::local::NormalizedLocalConfig::from(
		app.client.clone(),
		app.state.gateway(),
		yaml_content.as_str(),
	)
	.await
	{
		return Err(ErrorResponse::String(e.to_string()));
	}

	// Write the YAML content to the file
	fs_err::tokio::write(file_path, yaml_content)
		.await
		.map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	// Return success response
	Ok(Json(
		serde_json::json!({"status": "success", "message": "Configuration written successfully"}),
	))
}

/// Helper struct to extract registry config from local config YAML
#[derive(Deserialize)]
struct LocalConfigRegistry {
	#[serde(default)]
	registry: Option<RegistryConfigRef>,
}

#[derive(Deserialize)]
struct RegistryConfigRef {
	source: String,
}

/// Extract registry file path from the local config
async fn get_registry_path(app: &App) -> Result<PathBuf, ErrorResponse> {
	let config_source = app.cfg()?;
	let config_str = config_source.read_to_string().await?;

	let local_config: LocalConfigRegistry =
		yamlviajson::from_str(&config_str).map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	let registry_config = local_config
		.registry
		.ok_or_else(|| ErrorResponse::String("No registry configured in local config".to_string()))?;

	// Parse the source URI to extract file path
	let source = &registry_config.source;
	if source.starts_with("file://") {
		let path_str = source.strip_prefix("file://").unwrap();
		Ok(PathBuf::from(path_str))
	} else {
		Err(ErrorResponse::String(format!(
			"Registry source must be a file:// URI for UI editing, got: {}",
			source
		)))
	}
}

/// GET /registry - Fetch the current registry JSON
async fn get_registry(State(app): State<App>) -> Result<Json<Value>, ErrorResponse> {
	let registry_path = get_registry_path(&app).await?;

	// Check if registry file exists
	if !registry_path.exists() {
		// Return empty registry if file doesn't exist
		return Ok(Json(serde_json::json!({
			"schemaVersion": "2.0",
			"schemas": [],
			"servers": [],
			"agents": [],
			"tools": []
		})));
	}

	let content = fs_err::tokio::read_to_string(&registry_path)
		.await
		.map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	let registry: Value =
		serde_json::from_str(&content).map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	Ok(Json(registry))
}

/// POST /registry - Update the registry JSON
async fn write_registry(
	State(app): State<App>,
	Json(registry_json): Json<Value>,
) -> Result<Json<Value>, ErrorResponse> {
	let registry_path = get_registry_path(&app).await?;

	// Validate that the JSON is a valid registry by attempting to parse it
	// This ensures we don't write invalid data
	let _: crate::mcp::registry::types::Registry = serde_json::from_value(registry_json.clone())
		.map_err(|e| ErrorResponse::String(format!("Invalid registry format: {}", e)))?;

	// Create parent directory if it doesn't exist
	if let Some(parent) = registry_path.parent() {
		if !parent.exists() {
			fs_err::tokio::create_dir_all(parent)
				.await
				.map_err(|e| ErrorResponse::Anyhow(e.into()))?;
		}
	}

	// Write the registry JSON with pretty formatting
	let json_content = serde_json::to_string_pretty(&registry_json)
		.map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	fs_err::tokio::write(&registry_path, json_content)
		.await
		.map_err(|e| ErrorResponse::Anyhow(e.into()))?;

	Ok(Json(serde_json::json!({
		"status": "success",
		"message": "Registry written successfully",
		"path": registry_path.display().to_string()
	})))
}

pub fn add_cors_layer() -> CorsLayer {
	CorsLayer::new()
		.allow_origin(
			[
				"http://0.0.0.0:3000",
				"http://localhost:3000",
				"http://127.0.0.1:3000",
				"http://0.0.0.0:19000",
				"http://127.0.0.1:19000",
				"http://localhost:19000",
			]
			.map(|origin| origin.parse::<HeaderValue>().unwrap()),
		)
		.allow_headers([
			CONTENT_TYPE,
			AUTHORIZATION,
			HeaderName::from_static("x-requested-with"),
		])
		.allow_methods([
			Method::GET,
			Method::POST,
			Method::PUT,
			Method::DELETE,
			Method::OPTIONS,
		])
		.allow_credentials(true)
		.expose_headers([CONTENT_TYPE, CONTENT_LENGTH])
		.max_age(Duration::from_secs(3600))
}

impl AdminFallback for UiHandler {
	fn handle(&self, req: http::Request<Incoming>) -> AdminResponse {
		let router = self.router.clone();
		Box::pin(async { router.oneshot(req).await.unwrap() })
	}
}
