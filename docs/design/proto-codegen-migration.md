# Proto Codegen Migration Plan

## Goal

Migrate from hand-written types to proto-generated types in both Rust and TypeScript, using JSON serialization for human-readable registry files.

```
                    registry.proto
                    (Single Source of Truth)
                          │
            ┌─────────────┼─────────────┐
            ▼             ▼             ▼
        prost          ts-proto    JSON Schema
        + serde       (or buf)     (validation)
            │             │             │
            └─────────────┼─────────────┘
                          ▼
                   Proto3 JSON Mapping
                   (human-readable, portable)
```

## Current State (Updated 2026-01-25)

| Component | Location | Status |
|-----------|----------|--------|
| Proto definition | `crates/agentgateway/proto/registry.proto` | Complete (900+ lines, includes ConstructBinding) |
| Proto codegen | `crates/agentgateway/build.rs` | **pbjson generates proto3 JSON serde** |
| Proto types | `crates/agentgateway/src/types/proto.rs` | **Generated, compiles, tests passing** |
| Rust types | `crates/agentgateway/src/mcp/registry/types.rs` | Hand-written, still in use |
| Rust patterns | `crates/agentgateway/src/mcp/registry/patterns/*.rs` | Hand-written, still in use |
| Golden tests | `crates/agentgateway/src/mcp/registry/tests/golden_json.rs` | **21 tests passing** |
| TypeScript types | `packages/vmcp-dsl/src/types.ts` | Hand-written, pending migration |
| Visual builder | `packages/vmcp-dsl/tool-builder/index.html` | Inline JS, pending migration |

### Key Findings from Phases 0-4

1. **pbjson solves the oneof+serde problem**: prost's oneof enums don't support serde derives, but pbjson-build generates proper proto3 JSON serialization that handles oneofs correctly.

2. **Field naming differences between v1 and v2**:
   - `source.target` (v1) → `source.server` (v2)
   - Hand-written types accept both via `#[serde(alias)]`
   - Proto types use canonical v2 format only

3. **ConstructBinding was missing from proto**: Added it to enable complete DataBinding support (input, step, constant, construct).

## TDD Approach

### Phase 0: Golden Test Fixtures (Do First)

Capture the current JSON format as ground truth before any changes.

**Files to capture as fixtures:**

```bash
# Existing registry files that must continue to work
examples/ecommerce-demo/gateway-configs/ecommerce_registry_v2.json
examples/pattern-demos/configs/registry-v2-example.json
examples/pattern-demos/configs/registry.json

# Create synthetic fixtures covering all patterns
tests/fixtures/registry/
  ├── minimal.json           # Simplest valid registry
  ├── v1-basic.json          # v1 format with source tools
  ├── v2-full.json           # v2 with schemas, servers, agents
  ├── pipeline.json          # Pipeline pattern
  ├── scatter-gather.json    # Scatter-gather pattern
  ├── filter.json            # Filter pattern
  ├── schema-map.json        # SchemaMap pattern
  ├── map-each.json          # MapEach pattern
  ├── nested-patterns.json   # Patterns inside patterns
  └── stateful-patterns.json # Retry, timeout, cache, etc.
```

**Rust test (create first):**

```rust
// crates/agentgateway/src/mcp/registry/tests/golden_json.rs

#[cfg(test)]
mod golden_json_tests {
    use super::*;

    /// Golden test: Ensure JSON serialization is stable
    /// This test MUST pass before AND after migration
    #[test]
    fn test_roundtrip_minimal_registry() {
        let json = include_str!("../../../tests/fixtures/registry/minimal.json");
        let registry: Registry = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string_pretty(&registry).unwrap();

        // Parse both as Value to compare semantically (ignore whitespace)
        let original: serde_json::Value = serde_json::from_str(json).unwrap();
        let roundtripped: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(original, roundtripped);
    }

    // ... similar tests for each fixture
}
```

**TypeScript test (create first):**

```typescript
// packages/vmcp-dsl/src/__tests__/golden-json.test.ts

import { readFileSync } from 'fs';
import { Registry } from '../generated/registry'; // Will exist after codegen

describe('Golden JSON compatibility', () => {
  const fixtures = [
    'minimal.json',
    'v1-basic.json',
    'v2-full.json',
    // ...
  ];

  fixtures.forEach(fixture => {
    test(`roundtrip ${fixture}`, () => {
      const json = readFileSync(`../../tests/fixtures/registry/${fixture}`, 'utf-8');
      const parsed = JSON.parse(json);
      const registry = Registry.fromJSON(parsed);
      const reserialized = Registry.toJSON(registry);
      expect(reserialized).toEqual(parsed);
    });
  });
});
```

---

### Phase 1: Proto Codegen Infrastructure

#### 1.1 Rust: Configure prost for JSON compatibility

**Modify `crates/agentgateway/build.rs`:**

```rust
fn main() -> Result<(), anyhow::Error> {
    let proto_files = [
        // ... existing protos
        "proto/registry.proto",
    ];

    let config = {
        let mut c = prost_build::Config::new();
        c.disable_comments(Some("."));

        // Existing extern paths
        c.extern_path(".google.protobuf.Value", "::prost_wkt_types::Value");
        c.extern_path(".google.protobuf.Struct", "::prost_wkt_types::Struct");

        // NEW: Add serde derives for registry types
        c.type_attribute(
            "agentgateway.dev.registry",
            "#[derive(serde::Serialize, serde::Deserialize)]"
        );

        // NEW: Use camelCase for JSON compatibility with existing files
        c.type_attribute(
            "agentgateway.dev.registry",
            r#"#[serde(rename_all = "camelCase")]"#
        );

        // NEW: Handle proto3 optional fields correctly
        c.type_attribute(
            "agentgateway.dev.registry",
            "#[serde(skip_serializing_if = \"Option::is_none\")]"
        );

        c
    };

    // ... rest unchanged
}
```

**Add dependency in `Cargo.toml`:**

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
prost-wkt-types = { version = "0.7.0", features = ["vendored-protox", "serde"] }
```

#### 1.2 TypeScript: Set up buf/ts-proto

**Create `buf.yaml` in project root:**

```yaml
version: v1
breaking:
  use:
    - FILE
lint:
  use:
    - DEFAULT
```

**Create `buf.gen.yaml`:**

```yaml
version: v1
plugins:
  - plugin: ts-proto
    out: packages/vmcp-dsl/src/generated
    opt:
      - esModuleInterop=true
      - outputJsonMethods=true
      - outputEncodeMethods=false  # We only need JSON, not binary
      - stringEnums=true
      - useOptionals=messages
      - snakeToCamel=true
      - forceLong=string
```

**Add to `packages/vmcp-dsl/package.json`:**

```json
{
  "scripts": {
    "generate": "buf generate ../../crates/agentgateway/proto --path registry.proto",
    "build": "npm run generate && tsc",
    "test": "npm run generate && jest"
  },
  "devDependencies": {
    "@bufbuild/buf": "^1.28.0",
    "ts-proto": "^1.165.0"
  }
}
```

---

### Phase 2: Generate Types

#### 2.1 Rust: Include generated module

**Create `crates/agentgateway/src/mcp/registry/proto.rs`:**

```rust
//! Proto-generated registry types
//!
//! These types are generated from registry.proto and provide the canonical
//! type definitions. The JSON serialization uses proto3 JSON mapping with
//! camelCase field names for compatibility with existing registry files.

// Include the generated code
tonic::include_proto!("agentgateway.dev.registry");

// Re-export key types at module level
pub use registry::*;
pub use tool_definition::Implementation;
pub use pattern_spec::Pattern;
pub use data_binding::Source as DataBindingSource;
// ... other re-exports as needed
```

#### 2.2 TypeScript: Verify generation

```bash
cd packages/vmcp-dsl
npm run generate
```

This creates `src/generated/registry.ts` with types like:

```typescript
export interface Registry {
  schemaVersion: string;
  tools: ToolDefinition[];
  schemas: SchemaDefinition[];
  servers: ServerDefinition[];
  agents: AgentDefinition[];
}

export const Registry = {
  fromJSON(object: any): Registry { ... },
  toJSON(message: Registry): unknown { ... },
};
```

---

### Phase 3: Compatibility Tests

#### 3.1 Rust: Parallel type compatibility

**Create `crates/agentgateway/src/mcp/registry/tests/compat.rs`:**

```rust
//! Compatibility tests between hand-written and generated types
//! These tests ensure the migration doesn't break existing JSON files

#[cfg(test)]
mod compat_tests {
    use crate::mcp::registry::types as hand_written;
    use crate::mcp::registry::proto as generated;

    /// Test that both type systems parse the same JSON identically
    #[test]
    fn test_parse_compatibility() {
        let json = include_str!("../../../../tests/fixtures/registry/v2-full.json");

        // Parse with hand-written types
        let hw: hand_written::Registry = serde_json::from_str(json)
            .expect("hand-written types should parse");

        // Parse with generated types
        let gen: generated::Registry = serde_json::from_str(json)
            .expect("generated types should parse");

        // Compare key fields
        assert_eq!(hw.schema_version, gen.schema_version);
        assert_eq!(hw.tools.len(), gen.tools.len());
        assert_eq!(hw.schemas.len(), gen.schemas.len());
        // ... more field comparisons
    }

    /// Test that serialization produces equivalent JSON
    #[test]
    fn test_serialize_compatibility() {
        let json = include_str!("../../../../tests/fixtures/registry/v2-full.json");

        let hw: hand_written::Registry = serde_json::from_str(json).unwrap();
        let gen: generated::Registry = serde_json::from_str(json).unwrap();

        let hw_json: serde_json::Value = serde_json::to_value(&hw).unwrap();
        let gen_json: serde_json::Value = serde_json::to_value(&gen).unwrap();

        assert_eq!(hw_json, gen_json, "Serialized JSON should be identical");
    }
}
```

#### 3.2 TypeScript: Type compatibility

**Create `packages/vmcp-dsl/src/__tests__/compat.test.ts`:**

```typescript
import * as handWritten from '../types';
import * as generated from '../generated/registry';

describe('Type compatibility', () => {
  test('generated types are assignable to hand-written interfaces', () => {
    const genRegistry: generated.Registry = {
      schemaVersion: '2.0',
      tools: [],
      schemas: [],
      servers: [],
      agents: [],
    };

    // This should compile - generated types should satisfy hand-written interfaces
    const hwRegistry: handWritten.RegistryV2 = genRegistry as any;
    expect(hwRegistry.schemaVersion).toBe('2.0');
  });
});
```

---

### Phase 4: Migrate Rust Runtime

#### 4.1 Create adapter layer (incremental migration)

**Create `crates/agentgateway/src/mcp/registry/types_compat.rs`:**

```rust
//! Compatibility layer during migration
//!
//! This module provides conversion between hand-written and generated types,
//! allowing incremental migration of the codebase.

use crate::mcp::registry::proto as gen;
use crate::mcp::registry::types as hw;

impl From<gen::Registry> for hw::Registry {
    fn from(g: gen::Registry) -> Self {
        hw::Registry {
            schema_version: g.schema_version,
            tools: g.tools.into_iter().map(Into::into).collect(),
            // ... convert all fields
        }
    }
}

impl From<hw::Registry> for gen::Registry {
    fn from(h: hw::Registry) -> Self {
        gen::Registry {
            schema_version: h.schema_version,
            tools: h.tools.into_iter().map(Into::into).collect(),
            // ... convert all fields
        }
    }
}

// Implement for all nested types...
```

#### 4.2 Switch entry points one at a time

```rust
// In store.rs - change the parse entry point
pub fn load_registry(json: &str) -> Result<Registry, Error> {
    // OLD:
    // serde_json::from_str::<hw::Registry>(json)

    // NEW (during migration):
    let generated: gen::Registry = serde_json::from_str(json)?;
    Ok(generated.into())  // Convert to hand-written for now

    // FINAL (after migration complete):
    // serde_json::from_str::<gen::Registry>(json)
}
```

#### 4.3 Update executor to use generated types directly

Once adapters are tested, update each executor:

```rust
// executor/pipeline.rs
// Change from:
use crate::mcp::registry::patterns::PipelineSpec;
// To:
use crate::mcp::registry::proto::PipelineSpec;
```

#### 4.4 Delete hand-written types

Once all consumers are migrated:

1. Remove `types.rs`
2. Remove `patterns/*.rs` type definitions (keep executor logic)
3. Update `mod.rs` to export from `proto.rs`

---

### Phase 5: Migrate TypeScript DSL

#### 5.1 Update builders to construct generated types

**Modify `packages/vmcp-dsl/src/builder.ts`:**

```typescript
// OLD:
import { Registry, ToolDefinition } from './types';

// NEW:
import { Registry, ToolDefinition } from './generated/registry';
import type { Registry as RegistryType } from './types'; // Keep for backwards compat

export class RegistryBuilder {
  private tools: ToolDefinition[] = [];

  build(): Registry {
    return {
      schemaVersion: '2.0',
      tools: this.tools,
      schemas: [],
      servers: [],
      agents: [],
    };
  }

  // Add explicit JSON output method
  toJSON(): string {
    return JSON.stringify(Registry.toJSON(this.build()), null, 2);
  }
}
```

#### 5.2 Update compiler to use generated serialization

**Modify `packages/vmcp-dsl/src/compiler.ts`:**

```typescript
import { Registry } from './generated/registry';

export function compile(registry: Registry): string {
  // Use the generated toJSON for canonical serialization
  return JSON.stringify(Registry.toJSON(registry), null, 2);
}
```

#### 5.3 Deprecate hand-written types

**Modify `packages/vmcp-dsl/src/types.ts`:**

```typescript
/**
 * @deprecated Import from './generated/registry' instead
 * This file is maintained for backwards compatibility only.
 */
export * from './generated/registry';
```

---

### Phase 6: Update Visual Tool Builder

The visual builder (`tool-builder/index.html`) currently has inline type definitions in JavaScript. Update it to:

#### 6.1 Option A: Bundle generated types (recommended)

Create a browser bundle of the generated types:

**Add to `packages/vmcp-dsl/package.json`:**

```json
{
  "scripts": {
    "build:browser": "esbuild src/generated/registry.ts --bundle --outfile=tool-builder/registry-types.js --format=iife --global-name=RegistryTypes"
  }
}
```

**Update `tool-builder/index.html`:**

```html
<script src="registry-types.js"></script>
<script>
  const { Registry, ToolDefinition, PipelineSpec } = RegistryTypes;

  function generateJSON() {
    const registry = buildRegistryFromUI();
    // Use generated serialization
    return JSON.stringify(Registry.toJSON(registry), null, 2);
  }

  function validateRegistry(json) {
    try {
      const parsed = JSON.parse(json);
      Registry.fromJSON(parsed); // Throws if invalid
      return { valid: true, errors: [] };
    } catch (e) {
      return { valid: false, errors: [e.message] };
    }
  }
</script>
```

#### 6.2 Option B: Generate JSON Schema for validation

Use the proto to generate JSON Schema, then use that for validation:

```bash
# Using buf
buf generate --template buf.gen.jsonschema.yaml
```

```html
<script src="https://cdn.jsdelivr.net/npm/ajv@8/dist/ajv.bundle.min.js"></script>
<script>
  const ajv = new Ajv();
  const validate = ajv.compile(registrySchema);

  function validateRegistry(json) {
    const valid = validate(JSON.parse(json));
    return { valid, errors: validate.errors || [] };
  }
</script>
```

---

### Phase 7: Update Demos and Validate

#### 7.1 Verify existing registry files parse correctly

```bash
# Run the golden tests
cargo test golden_json

# Run TypeScript tests
cd packages/vmcp-dsl && npm test
```

#### 7.2 Update ecommerce-demo

```bash
# Test that the gateway loads the registry
RUST_LOG=debug cargo run -p agentgateway-app -- \
  -f examples/ecommerce-demo/gateway-configs/config.yaml

# Verify tools are exposed correctly
curl http://localhost:15000/mcp/tools/list
```

#### 7.3 Update pattern-demos

```bash
# Test each pattern demo
cd examples/pattern-demos
./test-patterns.sh
```

#### 7.4 Validate visual tool builder

1. Open `packages/vmcp-dsl/tool-builder/index.html` in browser
2. Load example configurations
3. Build a new tool composition
4. Export JSON
5. Load the exported JSON into the gateway
6. Verify it works

---

## Migration Checklist

### Phase 0: Golden Tests ✅ COMPLETE
- [x] Create `tests/fixtures/registry/` directory structure
- [x] Copy existing registry files as fixtures (10 fixtures)
- [x] Create synthetic fixtures for all patterns
- [x] Write Rust golden semantic tests (11 tests)
- [ ] Write TypeScript golden roundtrip tests (deferred to Phase 6)
- [x] Ensure all tests pass with current code

### Phase 1: Infrastructure ✅ COMPLETE
- [x] Update `build.rs` with pbjson-build for proto3 JSON serde
- [x] Add pbjson and pbjson-build dependencies
- [x] Set up buf.gen.yaml with ts-proto configuration
- [x] Add ts-proto to devDependencies
- [ ] Verify `npm run generate` works (pending)

### Phase 2: Solve Oneof+Serde Challenge ✅ COMPLETE
- [x] Identified that prost's oneof enums don't get serde derives
- [x] Solution: Use pbjson-build which generates proto3 JSON-compatible serde
- [x] Proto types now have full serde support with proper oneof handling
- [x] Added ConstructBinding to proto (was missing from hand-written types)

### Phase 3: Generate Types ✅ COMPLETE
- [x] Create `proto.rs` with include_proto and pbjson serde
- [x] Proto types compile and serialize correctly
- [x] All 6 proto type tests pass (minimal, source, pipeline, scatter-gather, roundtrip, construct)

### Phase 4: Compatibility Tests ✅ COMPLETE
- [x] Create Rust compat tests (4 tests documenting v1 vs v2 differences)
- [x] Document field naming differences (target→server, stepId→step_id)
- [x] Document proto3 JSON canonical format
- [x] Document migration path from v1 to v2
- [x] All 21 golden tests passing

### Phase 5: Rust Migration (PENDING)
- [ ] Create types_compat.rs adapter layer
- [ ] Switch store.rs to parse generated types
- [ ] Migrate each executor module
- [ ] Delete hand-written types.rs
- [ ] Delete hand-written patterns/*.rs types
- [ ] Update all imports

### Phase 6: TypeScript Migration (PENDING)
- [ ] Run buf generate for TypeScript
- [ ] Update builder.ts to use generated types
- [ ] Update compiler.ts to use generated serialization
- [ ] Add deprecation notice to types.ts
- [ ] Update pattern builders
- [ ] All existing tests still pass

### Phase 7: Visual Builder (PENDING)
- [ ] Create browser bundle of generated types
- [ ] Update index.html to use bundled types
- [ ] Update generateJSON() function
- [ ] Update validateRegistry() function
- [ ] Manual testing in browser

### Phase 8: Demos (PENDING)
- [ ] Gateway loads ecommerce-demo registry
- [ ] All pattern-demos work
- [ ] End-to-end agent tests pass
- [ ] Visual builder export → gateway load works

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Proto3 JSON mapping differs from current format | Golden tests catch this early; may need custom serde attributes |
| `google.protobuf.Struct` serialization differences | Use `prost-wkt-types` with serde feature; test thoroughly |
| Breaking changes to existing registry files | Golden tests + adapter layer allows incremental migration |
| Visual builder regression | Keep old inline types as fallback; manual testing |
| Performance regression from type conversions | Benchmark before removing adapter layer |

## Success Criteria

1. All existing registry JSON files parse without modification
2. Round-trip serialization produces identical JSON
3. Generated types are the only source of truth
4. Visual tool builder works with generated types
5. All demos pass
6. No runtime performance regression
