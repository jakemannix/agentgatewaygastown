use std::net;

use http::{status, uri};
use thiserror::Error;

#[allow(warnings)]
#[warn(clippy::derive_partial_eq_without_eq)]

pub mod istio {
	pub mod workload {
		tonic::include_proto!("istio.workload");
	}
}
pub mod workload {
	pub use super::istio::workload::*;
}

#[allow(warnings)]
#[warn(clippy::derive_partial_eq_without_eq)]
// Tonic is auto-generating weird imports for Istio, so build the module structure it expects but
// make our own module that de-nests it
pub mod agentgateway1 {
	pub mod agentgateway2 {
		pub mod agentgateway3 {
			tonic::include_proto!("agentgateway.dev.resource");
		}
	}
}
pub mod agent {
	pub use super::agentgateway1::agentgateway2::agentgateway3::*;
}

// Proto-generated registry types
// Package: agentgateway.dev.registry
//
// NOTE: Currently disabled due to serde+oneof complexity.
// The registry.proto uses many oneofs (PatternSpec, DataBinding, FieldSource, etc.)
// and prost's oneof enums don't automatically get serde derives from type_attribute.
//
// Options to fix:
// 1. Use prost-wkt-build with serde-serialize feature (requires oneof handling)
// 2. Use serde_with or custom deserialization for oneof types
// 3. Generate types without serde and add a manual conversion layer
//
// For now, continue using the hand-written types in mcp::registry::types
// which have proper serde handling for all patterns.
//
// TODO: Revisit in Phase 2 with prost-wkt or custom oneof serde handling
//
// #[allow(warnings)]
// pub mod registry_proto_gen {
// 	tonic::include_proto!("agentgateway.dev.registry");
// }

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum ProtoError {
	#[error("failed to parse namespaced hostname: {0}")]
	NamespacedHostnameParse(String),
	#[error("failed to parse address: {0}")]
	AddressParse(#[from] net::AddrParseError),
	#[error("failed to parse address, had {0} bytes")]
	ByteAddressParse(usize),
	#[error("invalid cidr: {0}")]
	PrefixParse(#[from] ipnet::PrefixLenError),
	#[error("unknown enum: {0}")]
	EnumParse(String),
	#[error("nonempty gateway address is missing address")]
	MissingGatewayAddress,
	#[error("decode error: {0}")]
	DecodeError(#[from] prost::DecodeError),
	#[error("decode error: {0}")]
	EnumError(#[from] prost::UnknownEnumValue),
	#[error("invalid URI: {0}")]
	InvalidURI(#[from] uri::InvalidUri),
	#[error("invalid status code: {0}")]
	InvalidStatusCode(#[from] status::InvalidStatusCode),
	#[error("error: {0}")]
	Generic(String),
	#[error("invalid header value: {0}")]
	HeaderValue(#[from] ::http::header::InvalidHeaderValue),
	#[error("invalid header name: {0}")]
	HeaderName(#[from] ::http::header::InvalidHeaderName),
	#[error("invalid regex: {0}")]
	Regex(#[from] regex::Error),
	#[error("invalid duration: {0}")]
	Duration(#[from] prost_types::DurationError),
	#[error("missing required field")]
	MissingRequiredField,
	#[error("invalid json: {0}")]
	Json(#[from] serde_json::Error),
}
