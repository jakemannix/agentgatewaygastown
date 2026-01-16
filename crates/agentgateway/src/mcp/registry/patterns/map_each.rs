// Map Each pattern types

use serde::{Deserialize, Serialize};

use super::PatternSpec;

/// MapEachSpec applies an operation to each element of an array
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEachSpec {
	/// The operation to apply to each element
	pub inner: MapEachInner,
}

impl MapEachSpec {
	/// Create a MapEach that calls a tool for each element
	pub fn tool(name: impl Into<String>) -> Self {
		Self { inner: MapEachInner::Tool(name.into()) }
	}

	/// Create a MapEach that applies a pattern for each element
	pub fn pattern(spec: PatternSpec) -> Self {
		Self { inner: MapEachInner::Pattern(Box::new(spec)) }
	}

	/// Get the names of tools referenced by this map-each
	pub fn referenced_tools(&self) -> Vec<&str> {
		self.inner.referenced_tools()
	}
}

/// The inner operation of a MapEach
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MapEachInner {
	/// Tool name to call for each element
	Tool(String),

	/// Pattern to apply for each element
	Pattern(Box<PatternSpec>),
}

impl MapEachInner {
	/// Get the names of tools referenced by this inner operation
	pub fn referenced_tools(&self) -> Vec<&str> {
		match self {
			MapEachInner::Tool(name) => vec![name.as_str()],
			MapEachInner::Pattern(p) => p.referenced_tools(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_map_each_tool() {
		let json = r#"{
			"inner": { "tool": "fetch_document" }
		}"#;

		let map_each: MapEachSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(map_each.inner, MapEachInner::Tool(ref name) if name == "fetch_document"));
	}

	#[test]
	fn test_parse_map_each_pattern() {
		let json = r#"{
			"inner": {
				"pattern": {
					"schemaMap": {
						"mappings": {
							"title": { "path": "$.name" }
						}
					}
				}
			}
		}"#;

		let map_each: MapEachSpec = serde_json::from_str(json).unwrap();
		assert!(matches!(map_each.inner, MapEachInner::Pattern(_)));
	}

	#[test]
	fn test_builder_tool() {
		let map_each = MapEachSpec::tool("my_tool");
		assert!(matches!(map_each.inner, MapEachInner::Tool(ref name) if name == "my_tool"));
	}

	#[test]
	fn test_referenced_tools() {
		let map_each = MapEachSpec::tool("fetch");
		let refs = map_each.referenced_tools();
		assert_eq!(refs, vec!["fetch"]);
	}
}

