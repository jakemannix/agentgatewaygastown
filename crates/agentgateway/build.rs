// This build script is used to generate the rust source files that
// we need for XDS GRPC communication and registry types.
fn main() -> Result<(), anyhow::Error> {
	let proto_files = [
		"proto/ext_proc.proto",
		"proto/ext_authz.proto",
		"proto/rls.proto",
		"proto/resource.proto",
		"proto/workload.proto",
		"proto/citadel.proto",
		"proto/registry.proto",
	]
	.iter()
	.map(|name| std::env::current_dir().unwrap().join(name))
	.collect::<Vec<_>>();
	let include_dirs = ["proto/"]
		.iter()
		.map(|i| std::env::current_dir().unwrap().join(i))
		.collect::<Vec<_>>();
	let config = {
		let mut c = prost_build::Config::new();
		c.disable_comments(Some("."));
		c.bytes([
			".istio.workload.Workload",
			".istio.workload.Service",
			".istio.workload.GatewayAddress",
			".istio.workload.Address",
		]);
		c.extern_path(".google.protobuf.Value", "::prost_wkt_types::Value");
		c.extern_path(".google.protobuf.Struct", "::prost_wkt_types::Struct");

		// Registry serde support - TODO: needs work for oneof handling
		//
		// The registry.proto uses many oneofs (PatternSpec, DataBinding, FieldSource, etc.)
		// and prost doesn't automatically add serde derives to oneof enum variants.
		// The type_attribute only applies to messages, not inner oneof enums.
		//
		// Options to fix:
		// 1. Use prost-wkt-build with proper oneof serde handling
		// 2. Use serde_with for externally tagged enum serialization
		// 3. Create a manual conversion layer from proto types
		//
		// For now, continue using the hand-written types in mcp::registry::types
		// which have proper serde handling via #[serde(flatten)] for oneofs.
		//
		// See docs/design/proto-codegen-migration.md for full plan.
		//
		// TODO(Phase 2): Enable once oneof serde is solved
		// c.type_attribute(
		// 	".agentgateway.dev.registry",
		// 	"#[derive(serde::Serialize, serde::Deserialize)]",
		// );
		// c.type_attribute(
		// 	".agentgateway.dev.registry",
		// 	"#[serde(rename_all = \"camelCase\")]",
		// );

		c
	};
	let fds = protox::compile(&proto_files, &include_dirs)?;
	tonic_prost_build::configure()
		.build_server(true)
		.compile_fds_with_config(fds, config)?;

	// This tells cargo to re-run this build script only when the proto files
	// we're interested in change or the any of the proto directories were updated.
	for path in [proto_files, include_dirs].concat() {
		println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
	}
	Ok(())
}
