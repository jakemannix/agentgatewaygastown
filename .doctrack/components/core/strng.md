---
feature: core
type: component
files:
  - crates/core/src/strng.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Strng (Interned Strings)

## Responsibility

Provide an efficient, immutable, reference-counted string type for use throughout the codebase, with Prometheus label encoding support.

## Internal Logic

### Strng Type

`Strng` is a type alias for `arcstr::ArcStr`:
- **8 bytes** (vs 24 bytes for `String`)
- **Cheap cloning** via reference counting
- **Immutable** after creation
- Supports `AsRef<str>`, `Display`, and conversion to `String`

### Helper Functions

- `strng::new(s)` - Creates a new `Strng` from anything `AsRef<str>`
- `strng::EMPTY` - Compile-time empty string constant
- `strng::format!` and `strng::literal!` - Re-exports from `arcstr`

### RichStrng Wrapper

`RichStrng` is a newtype wrapper around `Strng` that implements:
- `prometheus_client::EncodeLabelValue` - Encode as a Prometheus label value
- `prometheus_client::EncodeLabelKey` - Encode as a Prometheus label key
- `Deref<Target = Strng>` - Transparent access to underlying `Strng`
- `From<T> where T: Into<Strng>` - Convert from any string-like type

The wrapper exists because Rust's orphan rules prevent implementing foreign traits (`EncodeLabelValue`) on foreign types (`ArcStr`) directly.

### Design Notes

- The test verifies `size_of::<Strng>() == 8` as a compile-time assumption
- `ArcStr::strong_count` is available for debugging reference counts
- `literal!` creates compile-time interned strings with no runtime allocation

## Relationships

- **Used by**: [[components/core/metrics|metrics]] (label encoding), [[components/core/telemetry|telemetry]] (log fields), and throughout `crates/agentgateway` for configuration strings
- **Re-exported via**: [[components/core/prelude|prelude]]
- **Parent**: [[features/core|Core]]
