---
feature: core
type: component
files:
  - crates/core/src/version.rs
  - crates/core/build.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Version (Build Metadata)

## Responsibility

Capture and expose build-time metadata (version, git revision, Rust version, build profile, target) as a serializable struct.

## Internal Logic

### BuildInfo Struct

```rust
pub struct BuildInfo {
    pub version: &'static str,       // From VERSION env or build script
    pub git_revision: &'static str,  // Git SHA from report_build_info.sh
    pub rust_version: &'static str,  // From rustc_version crate
    pub build_profile: &'static str, // "debug" or "release"
    pub build_target: &'static str,  // e.g. "aarch64-apple-darwin"
}
```

All fields are `&'static str` populated at compile time via `env!()` macros reading cargo environment variables.

### Build Script (`build.rs`)

1. Runs `common/scripts/report_build_info.sh` (or `.ps1` on Windows)
2. Parses output lines in `key=value` format (e.g., `agentgateway.dev.buildGitRevision=abc123`)
3. Extracts the last segment of the key (after the last `.`) and sets `cargo:rustc-env=AGENTGATEWAY_BUILD_{key}={value}`
4. Additionally captures:
   - `RUSTC_VERSION` from `rustc_version::version()`
   - `PROFILE_NAME` from the OUT_DIR path
   - `TARGET` from cargo's TARGET env var
5. `cargo:rerun-if-env-changed=VERSION` ensures rebuilds when version changes

### Display

`BuildInfo` implements `Display` via `serde_json::to_string_pretty`, producing a formatted JSON output suitable for `--version` flags.

## Relationships

- **Used by**: `crates/agentgateway-app` for startup logging and version endpoint
- **Parent**: [[features/core|Core]]
