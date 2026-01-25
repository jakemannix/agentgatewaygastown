// This build script generates Rust types from protobuf definitions.
// - prost: generates base Rust types from proto
// - pbjson: generates proto3 JSON serde implementations (handles oneofs correctly)

use prost::Message;

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

	// Configure prost for base type generation
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
		c
	};

	// Compile protos with prost (generates base types)
	let fds = protox::compile(&proto_files, &include_dirs)?;
	tonic_prost_build::configure()
		.build_server(true)
		.compile_fds_with_config(fds.clone(), config)?;

	// Generate proto3 JSON serde implementations for registry types using pbjson
	// This handles oneofs correctly according to the proto3 JSON spec
	let out_dir = std::env::var("OUT_DIR")?;
	pbjson_build::Builder::new()
		.register_descriptors(&fds.encode_to_vec())?
		.build(&[".agentgateway.dev.registry"])?;

	// Also write the descriptor set for potential runtime use
	std::fs::write(
		std::path::Path::new(&out_dir).join("registry_descriptor.bin"),
		fds.encode_to_vec(),
	)?;

	// Tell cargo to re-run when protos change
	for path in [proto_files, include_dirs].concat() {
		println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
	}

	Ok(())
}
