use agent_core::trcng;
use futures_core::Stream;
use futures_util::StreamExt;
use http::StatusCode;
use http::request::Parts;
use itertools::Itertools;
use opentelemetry::global::BoxedSpan;
use opentelemetry::trace::{SpanContext, SpanKind, TraceContextExt, TraceState};
use opentelemetry::{Context, TraceFlags};
use rmcp::ErrorData;
use rmcp::model::{
	ClientNotification, ClientRequest, Implementation, JsonRpcNotification, JsonRpcRequest,
	ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, Prompt,
	PromptsCapability, ProtocolVersion, RequestId, ResourcesCapability, ServerCapabilities,
	ServerInfo, ServerJsonRpcMessage, ServerResult, Tool, ToolsCapability,
};
use std::borrow::Cow;
use std::sync::Arc;

use crate::cel::ContextBuilder;
use crate::http::Response;
use crate::http::jwt::Claims;
use crate::http::sessionpersistence::MCPSession;
use crate::mcp::mergestream::MergeFn;
use crate::mcp::rbac::{Identity, McpAuthorizationSet};
use crate::mcp::registry::RegistryStoreRef;
use crate::mcp::router::McpBackendGroup;
use crate::mcp::streamablehttp::ServerSseMessage;
use crate::mcp::upstream::{IncomingRequestContext, UpstreamError};
use crate::mcp::{ClientError, MCPInfo, mergestream, rbac, upstream};
use crate::proxy::httpproxy::PolicyClient;
use crate::telemetry::log::AsyncLog;
use crate::telemetry::trc::TraceParent;

const DELIMITER: &str = "_";

/// Result of resolving a tool call, which may be a virtual tool or composition
#[derive(Debug, Clone)]
pub enum ResolvedToolCall {
	/// A tool call that routes to a backend
	Backend {
		/// The target service/backend to route the call to
		target: String,
		/// The actual tool name on the backend
		tool_name: String,
		/// The arguments with defaults injected
		args: serde_json::Value,
		/// If this was a virtual tool, the original virtual name (for output transformation)
		virtual_name: Option<String>,
	},
	/// A composition that needs to be executed locally
	Composition {
		/// The composition name
		name: String,
		/// The arguments
		args: serde_json::Value,
	},
}

fn resource_name(default_target_name: Option<&String>, target: &str, name: &str) -> String {
	if default_target_name.is_none() {
		format!("{target}{DELIMITER}{name}")
	} else {
		name.to_string()
	}
}

#[derive(Debug, Clone)]
pub struct Relay {
	upstreams: Arc<upstream::UpstreamGroup>,
	pub policies: McpAuthorizationSet,
	// If we have 1 target only, we don't prefix everything with 'target_'.
	// Else this is empty
	default_target_name: Option<String>,
	is_multiplexing: bool,
	/// Optional tool registry for virtual tool mappings
	registry: Option<RegistryStoreRef>,
}

impl Relay {
	pub fn new(
		backend: McpBackendGroup,
		policies: McpAuthorizationSet,
		client: PolicyClient,
	) -> anyhow::Result<Self> {
		let mut is_multiplexing = false;
		let default_target_name = if backend.targets.len() != 1 {
			is_multiplexing = true;
			None
		} else if backend.targets[0].always_use_prefix {
			None
		} else {
			Some(backend.targets[0].name.to_string())
		};
		Ok(Self {
			upstreams: Arc::new(upstream::UpstreamGroup::new(client, backend)?),
			policies,
			default_target_name,
			is_multiplexing,
			registry: None,
		})
	}

	/// Create a Relay with a registry for virtual tool mappings
	pub fn with_registry(mut self, registry: RegistryStoreRef) -> Self {
		self.registry = Some(registry);
		self
	}

	/// Get the registry reference
	pub fn registry(&self) -> Option<&RegistryStoreRef> {
		self.registry.as_ref()
	}

	/// Resolve a tool call, handling virtual tools, compositions, and regular tools.
	///
	/// Returns a ResolvedToolCall which is either:
	/// - Backend: routes to a backend service
	/// - Composition: needs local execution via CompositionExecutor
	///
	/// For virtual tools, this will:
	/// - Map the virtual name to the source target and tool
	/// - Inject default arguments
	///
	/// For compositions, this returns the composition name for local execution.
	///
	/// For regular tools, this delegates to parse_resource_name.
	pub fn resolve_tool_call(
		&self,
		tool_name: &str,
		args: serde_json::Value,
	) -> Result<ResolvedToolCall, UpstreamError> {
		// First, check if this is a virtual tool or composition
		if let Some(ref reg) = self.registry {
			let guard = reg.get();
			if let Some(ref compiled_registry) = **guard {
				if let Some(tool) = compiled_registry.get_tool(tool_name) {
					// Check if this is a composition
					if tool.is_composition() {
						tracing::debug!(
							target: "virtual_tools",
							composition = tool_name,
							"resolved tool as composition"
						);
						return Ok(ResolvedToolCall::Composition {
							name: tool_name.to_string(),
							args,
						});
					}

					// This is a source-based virtual tool - resolve to backend
					if let Some(source_info) = tool.source_info() {
						let target = source_info.source.target.clone();
						let backend_tool = source_info.source.tool.clone();

						tracing::debug!(
							target: "virtual_tools",
							virtual_tool = tool_name,
							backend_target = %target,
							backend_tool = %backend_tool,
							"resolved virtual tool to backend"
						);

						// Inject defaults
						let transformed_args = tool
							.inject_defaults(args)
							.map_err(|e| UpstreamError::InvalidRequest(e.to_string()))?;

						return Ok(ResolvedToolCall::Backend {
							target,
							tool_name: backend_tool,
							args: transformed_args,
							virtual_name: Some(tool_name.to_string()),
						});
					}
				}
			}
		}

		// Not a virtual tool or composition - parse normally
		let (service_name, actual_tool) = self.parse_resource_name(tool_name)?;
		Ok(ResolvedToolCall::Backend {
			target: service_name.to_string(),
			tool_name: actual_tool.to_string(),
			args,
			virtual_name: None,
		})
	}

	/// Check if a tool is a composition
	pub fn is_composition(&self, tool_name: &str) -> bool {
		if let Some(ref reg) = self.registry {
			let guard = reg.get();
			if let Some(ref compiled_registry) = **guard {
				return compiled_registry.is_composition(tool_name);
			}
		}
		false
	}

	/// Transform tool output for virtual tools
	pub fn transform_tool_output(
		&self,
		virtual_name: &str,
		response: serde_json::Value,
	) -> Result<serde_json::Value, UpstreamError> {
		if let Some(ref reg) = self.registry {
			let guard = reg.get();
			if let Some(ref compiled_registry) = **guard {
				return compiled_registry
					.transform_output(virtual_name, response)
					.map_err(|e| UpstreamError::InvalidRequest(e.to_string()));
			}
		}
		Ok(response)
	}

	pub fn parse_resource_name<'a, 'b: 'a>(
		&'a self,
		res: &'b str,
	) -> Result<(&'a str, &'b str), UpstreamError> {
		if let Some(default) = self.default_target_name.as_ref() {
			Ok((default.as_str(), res))
		} else {
			res
				.split_once(DELIMITER)
				.ok_or(UpstreamError::InvalidRequest(
					"invalid resource name".to_string(),
				))
		}
	}

	/// Invoke a tool on a specific target and return the result as JSON.
	/// This is used by the composition executor to call backend tools.
	pub async fn invoke_tool(
		&self,
		target: &str,
		tool_name: &str,
		args: serde_json::Value,
		ctx: &IncomingRequestContext,
	) -> Result<serde_json::Value, UpstreamError> {
		use futures_util::StreamExt;

		// Get the upstream
		let upstream = self
			.upstreams
			.get(target)
			.map_err(|_| UpstreamError::InvalidRequest(format!("unknown service {}", target)))?;

		// Build the request
		let call_params = rmcp::model::CallToolRequestParam {
			name: tool_name.to_string().into(),
			arguments: args.as_object().cloned(),
		};

		// Use i32 to avoid floating-point precision issues when JSON serializes large numbers
		let request_id = RequestId::Number(rand::random::<i32>().abs() as i64);

		// Create a proper JsonRpcRequest using rmcp's types
		let call_tool_request = rmcp::model::CallToolRequest {
			method: Default::default(),
			params: call_params,
			extensions: Default::default(),
		};

		let request: JsonRpcRequest<ClientRequest> = JsonRpcRequest {
			jsonrpc: Default::default(),
			id: request_id.clone(),
			request: ClientRequest::CallToolRequest(call_tool_request),
		};

		// Send the request and get the response stream
		let mut stream = upstream.generic_stream(request, ctx).await?;

		// Get the first message from the stream
		let response = stream
			.next()
			.await
			.ok_or_else(|| UpstreamError::InvalidRequest("No response from tool call".to_string()))?
			.map_err(|e| UpstreamError::InvalidRequest(format!("Tool call error: {}", e)))?;

		// Extract the result from the JSON-RPC response
		match response {
			ServerJsonRpcMessage::Response(resp) => {
				// Extract the actual content from CallToolResult
				use rmcp::model::ServerResult;
				match resp.result {
					ServerResult::CallToolResult(ctr) => {
						// Find text content and try to parse as JSON
						for content in &ctr.content {
							if let rmcp::model::RawContent::Text(t) = &content.raw {
								// Try to parse as JSON, fall back to raw text
								if let Ok(json) = serde_json::from_str::<serde_json::Value>(&t.text) {
									return Ok(json);
								} else {
									// Return as string value if not valid JSON
									return Ok(serde_json::Value::String(t.text.clone()));
								}
							}
						}
						// No text content found, return null
						Ok(serde_json::Value::Null)
					},
					other => {
						// For other result types, serialize as-is
						serde_json::to_value(&other).map_err(|e| {
							UpstreamError::InvalidRequest(format!("Failed to serialize result: {}", e))
						})
					},
				}
			},
			ServerJsonRpcMessage::Error(err) => Err(UpstreamError::InvalidRequest(format!(
				"Tool call failed: {}",
				err.error.message
			))),
			_ => Err(UpstreamError::InvalidRequest(
				"Unexpected response type from tool call".to_string(),
			)),
		}
	}
}

// =============================================================================
// RelayToolInvoker - Real ToolInvoker implementation using Relay
// =============================================================================

use crate::mcp::registry::executor::{ExecutionError, ToolInvoker};

/// A ToolInvoker implementation that uses the Relay to make real backend calls.
/// This is used by the CompositionExecutor to invoke tools during composition execution.
pub struct RelayToolInvoker {
	relay: Arc<Relay>,
	ctx: IncomingRequestContext,
}

impl RelayToolInvoker {
	/// Create a new RelayToolInvoker
	pub fn new(relay: Arc<Relay>, ctx: IncomingRequestContext) -> Self {
		Self { relay, ctx }
	}
}

#[async_trait::async_trait]
impl ToolInvoker for RelayToolInvoker {
	async fn invoke(
		&self,
		tool_name: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value, ExecutionError> {
		// Resolve the tool call (handles virtual tools, compositions, and backend tools)
		let resolved = self
			.relay
			.resolve_tool_call(tool_name, args)
			.map_err(|e| ExecutionError::ToolExecutionFailed(e.to_string()))?;

		match resolved {
			ResolvedToolCall::Backend {
				target,
				tool_name: backend_tool,
				args,
				virtual_name,
			} => {
				// Use the Relay's invoke_tool method which handles the MCP protocol properly
				let result = self
					.relay
					.invoke_tool(&target, &backend_tool, args, &self.ctx)
					.await
					.map_err(|e| ExecutionError::ToolExecutionFailed(e.to_string()))?;

				// Apply output transformation if this was a virtual tool
				if let Some(vname) = virtual_name {
					self
						.relay
						.transform_tool_output(&vname, result)
						.map_err(|e| ExecutionError::ToolExecutionFailed(e.to_string()))
				} else {
					Ok(result)
				}
			},
			ResolvedToolCall::Composition { name, .. } => {
				// Nested compositions not yet supported
				Err(ExecutionError::ToolExecutionFailed(format!(
					"Nested composition '{}' not supported - compositions can only call backend tools",
					name
				)))
			},
		}
	}
}

impl Relay {
	pub fn get_sessions(&self) -> Option<Vec<MCPSession>> {
		let mut sessions = Vec::with_capacity(self.upstreams.size());
		for (_, us) in self.upstreams.iter_named() {
			sessions.push(us.get_session_state()?);
		}
		Some(sessions)
	}

	pub fn set_sessions(&self, sessions: Vec<MCPSession>) {
		for ((_, us), session) in self.upstreams.iter_named().zip(sessions) {
			us.set_session_id(&session.session, session.backend);
		}
	}
	pub fn count(&self) -> usize {
		self.upstreams.size()
	}

	pub fn is_multiplexing(&self) -> bool {
		self.is_multiplexing
	}
	pub fn default_target_name(&self) -> Option<String> {
		self.default_target_name.clone()
	}

	pub fn merge_tools(&self, cel: Arc<ContextBuilder>) -> Box<MergeFn> {
		let policies = self.policies.clone();
		let default_target_name = self.default_target_name.clone();
		// Clone registry reference for use in closure
		let registry = self.registry.clone();

		Box::new(move |streams| {
			// Collect all tools with their server names
			let backend_tools: Vec<(String, Tool)> = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let tools = match s {
						ServerResult::ListToolsResult(ltr) => ltr.tools,
						_ => vec![],
					};
					tools
						.into_iter()
						.map(|t| (server_name.to_string(), t))
						.collect_vec()
				})
				.collect_vec();

			// Apply registry transformations if configured
			let transformed_tools = if let Some(ref reg) = registry {
				let guard = reg.get();
				if let Some(ref compiled_registry) = **guard {
					compiled_registry.transform_tools(backend_tools)
				} else {
					backend_tools
				}
			} else {
				backend_tools
			};

			// Apply authorization policies and multiplexing renaming
			let tools = transformed_tools
				.into_iter()
				.filter(|(server_name, t)| {
					policies.validate(
						&rbac::ResourceType::Tool(rbac::ResourceId::new(
							server_name.to_string(),
							t.name.to_string(),
						)),
						&cel,
					)
				})
				// Rename to handle multiplexing
				.map(|(server_name, t)| Tool {
					name: Cow::Owned(resource_name(
						default_target_name.as_ref(),
						server_name.as_str(),
						&t.name,
					)),
					..t
				})
				.collect_vec();

			Ok(
				ListToolsResult {
					tools,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}

	pub fn merge_initialize(&self, pv: ProtocolVersion) -> Box<MergeFn> {
		Box::new(move |s| {
			if s.len() == 1 {
				let (_, ServerResult::InitializeResult(ir)) = s.into_iter().next().unwrap() else {
					return Ok(Self::get_info(pv).into());
				};
				return Ok(ir.clone().into());
			}

			let lowest_version = s
				.into_iter()
				.flat_map(|(_, v)| match v {
					ServerResult::InitializeResult(r) => Some(r.protocol_version),
					_ => None,
				})
				.min_by_key(|i| i.to_string())
				.unwrap_or(pv);
			// For now, we just send our own info. In the future, we should merge the results from each upstream.
			Ok(Self::get_info(lowest_version).into())
		})
	}

	pub fn merge_prompts(&self, cel: Arc<ContextBuilder>) -> Box<MergeFn> {
		let policies = self.policies.clone();
		let default_target_name = self.default_target_name.clone();
		Box::new(move |streams| {
			let prompts = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let prompts = match s {
						ServerResult::ListPromptsResult(lpr) => lpr.prompts,
						_ => vec![],
					};
					prompts
						.into_iter()
						.filter(|p| {
							policies.validate(
								&rbac::ResourceType::Prompt(rbac::ResourceId::new(
									server_name.to_string(),
									p.name.to_string(),
								)),
								&cel,
							)
						})
						.map(|p| Prompt {
							name: resource_name(default_target_name.as_ref(), server_name.as_str(), &p.name),
							..p
						})
						.collect_vec()
				})
				.collect_vec();
			Ok(
				ListPromptsResult {
					prompts,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}
	pub fn merge_resources(&self, cel: Arc<ContextBuilder>) -> Box<MergeFn> {
		let policies = self.policies.clone();
		Box::new(move |streams| {
			let resources = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let resources = match s {
						ServerResult::ListResourcesResult(lrr) => lrr.resources,
						_ => vec![],
					};
					resources
						.into_iter()
						.filter(|r| {
							policies.validate(
								&rbac::ResourceType::Resource(rbac::ResourceId::new(
									server_name.to_string(),
									r.uri.to_string(),
								)),
								&cel,
							)
						})
						// TODO(https://github.com/agentgateway/agentgateway/issues/404) map this to the service name,
						// if we add support for multiple services.
						.collect_vec()
				})
				.collect_vec();
			Ok(
				ListResourcesResult {
					resources,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}
	pub fn merge_resource_templates(&self, cel: Arc<ContextBuilder>) -> Box<MergeFn> {
		let policies = self.policies.clone();
		Box::new(move |streams| {
			let resource_templates = streams
				.into_iter()
				.flat_map(|(server_name, s)| {
					let resource_templates = match s {
						ServerResult::ListResourceTemplatesResult(lrr) => lrr.resource_templates,
						_ => vec![],
					};
					resource_templates
						.into_iter()
						.filter(|rt| {
							policies.validate(
								&rbac::ResourceType::Resource(rbac::ResourceId::new(
									server_name.to_string(),
									rt.uri_template.to_string(),
								)),
								&cel,
							)
						})
						// TODO(https://github.com/agentgateway/agentgateway/issues/404) map this to the service name,
						// if we add support for multiple services.
						.collect_vec()
				})
				.collect_vec();
			Ok(
				ListResourceTemplatesResult {
					resource_templates,
					next_cursor: None,
					meta: None,
				}
				.into(),
			)
		})
	}
	pub fn merge_empty(&self) -> Box<MergeFn> {
		Box::new(move |_| Ok(rmcp::model::ServerResult::empty(())))
	}
	pub async fn send_single(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
		service_name: &str,
	) -> Result<Response, UpstreamError> {
		let id = r.id.clone();
		let Ok(us) = self.upstreams.get(service_name) else {
			return Err(UpstreamError::InvalidRequest(format!(
				"unknown service {service_name}"
			)));
		};
		let stream = us.generic_stream(r, &ctx).await?;

		messages_to_response(id, stream)
	}

	/// Send to a single service with output transformation for virtual tools
	pub async fn send_single_with_output_transform(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
		service_name: &str,
		virtual_name: Option<String>,
	) -> Result<Response, UpstreamError> {
		tracing::debug!(
			target: "virtual_tools",
			service = service_name,
			virtual_name = ?virtual_name,
			"sending tool call to backend"
		);

		let id = r.id.clone();
		let Ok(us) = self.upstreams.get(service_name) else {
			tracing::warn!(
				target: "virtual_tools",
				service = service_name,
				"backend service not found in upstreams"
			);
			return Err(UpstreamError::InvalidRequest(format!(
				"unknown service {service_name}"
			)));
		};
		let stream = us.generic_stream(r, &ctx).await?;

		// If we have a virtual name and registry, transform the output
		if let Some(vname) = virtual_name {
			if let Some(ref reg) = self.registry {
				let reg_clone = reg.clone();
				let stream =
					stream.map(move |msg| msg.map(|m| transform_server_message(m, &vname, &reg_clone)));
				return messages_to_response(id, stream);
			}
		}

		messages_to_response(id, stream)
	}
	// For some requests, we don't have a sane mapping of incoming requests to a specific
	// downstream service when multiplexing. Only forward when we have only one backend.
	pub async fn send_single_without_multiplexing(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		let Some(service_name) = &self.default_target_name else {
			return Err(UpstreamError::InvalidMethod(r.request.method().to_string()));
		};
		self.send_single(r, ctx, service_name).await
	}
	pub async fn send_fanout_deletion(
		&self,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		for (_, con) in self.upstreams.iter_named() {
			con.delete(&ctx).await?;
		}
		Ok(accepted_response())
	}
	pub async fn send_fanout_get(
		&self,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		let mut streams = Vec::new();
		for (name, con) in self.upstreams.iter_named() {
			streams.push((name, con.get_event_stream(&ctx).await?));
		}

		let ms = mergestream::MergeStream::new_without_merge(streams);
		messages_to_response(RequestId::Number(0), ms)
	}
	pub async fn send_fanout(
		&self,
		r: JsonRpcRequest<ClientRequest>,
		ctx: IncomingRequestContext,
		merge: Box<MergeFn>,
	) -> Result<Response, UpstreamError> {
		let id = r.id.clone();
		let mut streams = Vec::new();
		for (name, con) in self.upstreams.iter_named() {
			streams.push((name, con.generic_stream(r.clone(), &ctx).await?));
		}

		let ms = mergestream::MergeStream::new(streams, id.clone(), merge);
		messages_to_response(id, ms)
	}
	pub async fn send_notification(
		&self,
		r: JsonRpcNotification<ClientNotification>,
		ctx: IncomingRequestContext,
	) -> Result<Response, UpstreamError> {
		let mut streams = Vec::new();
		for (name, con) in self.upstreams.iter_named() {
			streams.push((
				name,
				con
					.generic_notification(r.notification.clone(), &ctx)
					.await?,
			));
		}

		Ok(accepted_response())
	}
	fn get_info(pv: ProtocolVersion) -> ServerInfo {
		ServerInfo {
            protocol_version: pv,
            capabilities: ServerCapabilities {
                completions: None,
                experimental: None,
                logging: None,
                prompts: Some(PromptsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                tools: Some(ToolsCapability::default()),
            },
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server is a gateway to a set of mcp servers. It is responsible for routing requests to the correct server and aggregating the results.".to_string(),
            ),
        }
	}
}

pub fn setup_request_log(
	http: &Parts,
	span_name: &str,
) -> (BoxedSpan, AsyncLog<MCPInfo>, Arc<ContextBuilder>) {
	let traceparent = http.extensions.get::<TraceParent>();
	let mut ctx = Context::new();
	if let Some(tp) = traceparent {
		ctx = ctx.with_remote_span_context(SpanContext::new(
			tp.trace_id.into(),
			tp.span_id.into(),
			TraceFlags::new(tp.flags),
			true,
			TraceState::default(),
		));
	}
	let claims = http.extensions.get::<Claims>();

	let log = http
		.extensions
		.get::<AsyncLog<MCPInfo>>()
		.cloned()
		.unwrap_or_default();

	let cel = http
		.extensions
		.get::<Arc<ContextBuilder>>()
		.cloned()
		.expect("CelContextBuilder must be set");

	let tracer = trcng::get_tracer();
	let _span = trcng::start_span(span_name.to_string(), &Identity::new(claims.cloned()))
		.with_kind(SpanKind::Server)
		.start_with_context(tracer, &ctx);
	(_span, log, cel)
}

pub(crate) fn messages_to_response(
	id: RequestId,
	stream: impl Stream<Item = Result<ServerJsonRpcMessage, ClientError>> + Send + 'static,
) -> Result<Response, UpstreamError> {
	use futures_util::StreamExt;
	use rmcp::model::ServerJsonRpcMessage;
	let stream = stream.map(move |rpc| {
		let r = match rpc {
			Ok(rpc) => rpc,
			Err(e) => {
				ServerJsonRpcMessage::error(ErrorData::internal_error(e.to_string(), None), id.clone())
			},
		};
		// TODO: is it ok to have no event_id here?
		ServerSseMessage {
			event_id: None,
			message: Arc::new(r),
		}
	});
	Ok(crate::mcp::session::sse_stream_response(stream, None))
}

fn accepted_response() -> Response {
	::http::Response::builder()
		.status(StatusCode::ACCEPTED)
		.body(crate::http::Body::empty())
		.expect("valid response")
}

/// Transform a server message if it contains a tool call result
fn transform_server_message(
	msg: ServerJsonRpcMessage,
	virtual_name: &str,
	registry: &RegistryStoreRef,
) -> ServerJsonRpcMessage {
	use rmcp::model::ServerResult;

	// Only transform response messages
	let ServerJsonRpcMessage::Response(resp) = msg else {
		return msg;
	};

	// Check if it's a CallToolResult
	let ServerResult::CallToolResult(call_result) = resp.result else {
		return ServerJsonRpcMessage::Response(resp);
	};

	tracing::debug!(
		target: "virtual_tools",
		virtual_name,
		"processing tool result for output transformation"
	);

	// Try to transform the result content
	let guard = registry.get();
	let Some(ref compiled) = **guard else {
		tracing::debug!(target: "virtual_tools", "no compiled registry");
		return ServerJsonRpcMessage::Response(rmcp::model::JsonRpcResponse {
			result: ServerResult::CallToolResult(call_result),
			..resp
		});
	};

	// Get the tool to check if it has output transformation
	let Some(tool) = compiled.get_tool(virtual_name) else {
		tracing::debug!(target: "virtual_tools", virtual_name, "tool not found in registry");
		return ServerJsonRpcMessage::Response(rmcp::model::JsonRpcResponse {
			result: ServerResult::CallToolResult(call_result),
			..resp
		});
	};

	// If no output transform defined, pass through
	if !tool.has_output_transform() {
		tracing::debug!(target: "virtual_tools", virtual_name, "no output_transform defined, passing through");
		return ServerJsonRpcMessage::Response(rmcp::model::JsonRpcResponse {
			result: ServerResult::CallToolResult(call_result),
			..resp
		});
	}

	tracing::debug!(
		target: "virtual_tools",
		virtual_name,
		output_fields = ?tool.output_transform_fields(),
		"attempting output transformation"
	);

	// Try to transform the call result
	if let Some(transformed) = transform_call_tool_result(&call_result, tool) {
		tracing::debug!(target: "virtual_tools", virtual_name, "output transformation succeeded");
		return ServerJsonRpcMessage::Response(rmcp::model::JsonRpcResponse {
			result: ServerResult::CallToolResult(transformed),
			..resp
		});
	}

	tracing::debug!(target: "virtual_tools", virtual_name, "output transformation failed, returning original");
	// Fallback - return original
	ServerJsonRpcMessage::Response(rmcp::model::JsonRpcResponse {
		result: ServerResult::CallToolResult(call_result),
		..resp
	})
}

/// Transform a CallToolResult using the tool's output schema
fn transform_call_tool_result(
	result: &rmcp::model::CallToolResult,
	tool: &crate::mcp::registry::CompiledVirtualTool,
) -> Option<rmcp::model::CallToolResult> {
	use rmcp::model::{Annotated, RawContent, RawTextContent};

	// Find text content to transform
	let text_content = result.content.iter().find_map(|c| {
		if let RawContent::Text(t) = &c.raw {
			Some(t.text.as_str())
		} else {
			None
		}
	});

	let Some(text_content) = text_content else {
		tracing::debug!(
			target: "virtual_tools",
			content_types = ?result.content.iter().map(|c| match &c.raw {
				RawContent::Text(_) => "text",
				RawContent::Image(_) => "image",
				RawContent::Resource(_) => "resource",
				_ => "other",
			}).collect::<Vec<_>>(),
			"no text content found in result"
		);
		return None;
	};

	// Try to parse as JSON
	let json_value: serde_json::Value = match serde_json::from_str(text_content) {
		Ok(v) => v,
		Err(e) => {
			tracing::debug!(
				target: "virtual_tools",
				error = %e,
				text_preview = %text_content.chars().take(200).collect::<String>(),
				"failed to parse result as JSON"
			);
			return None;
		},
	};

	// Transform using the tool's output transformation
	let transformed = match tool.transform_output(json_value) {
		Ok(v) => v,
		Err(e) => {
			tracing::debug!(
				target: "virtual_tools",
				error = %e,
				"output transformation failed"
			);
			return None;
		},
	};

	tracing::debug!(
		target: "virtual_tools",
		"successfully transformed output"
	);

	// Create new result with both text content and structuredContent
	let new_content = vec![Annotated {
		raw: RawContent::Text(RawTextContent {
			text: serde_json::to_string_pretty(&transformed).unwrap_or_default(),
			meta: None,
		}),
		annotations: None,
	}];

	Some(rmcp::model::CallToolResult {
		content: new_content,
		structured_content: Some(transformed),
		is_error: result.is_error,
		meta: result.meta.clone(),
	})
}
