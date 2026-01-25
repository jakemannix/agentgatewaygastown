# Registry Test Fixtures

These fixtures test JSON parsing and serialization for the registry format.

## Implementation Status

### Implemented (Runtime Works)

These patterns have full runtime support:

| Pattern | Fixture | Status |
|---------|---------|--------|
| Source Tool | `v1-source-tools.json` | ✅ Stable |
| Pipeline | `pipeline.json` | ✅ Stable |
| Scatter-Gather | `scatter-gather.json` | ✅ Stable |
| Filter | `filter.json` | ✅ Stable |
| SchemaMap | `schema-map.json` | ✅ Stable |
| MapEach | `map-each.json` | ✅ Stable |
| Output Transform | `output-transform.json` | ✅ Stable |
| Registry v2 | `v2-full.json` | ✅ Stable |

### IR Only (No Runtime)

These patterns have IR/types defined but **NO RUNTIME EXECUTOR**:

| Pattern | Fixture | Status |
|---------|---------|--------|
| Retry | `stateful-patterns.json` | ⚠️ Parse only |
| Timeout | `stateful-patterns.json` | ⚠️ Parse only |
| Cache | `stateful-patterns.json` | ⚠️ Parse only |
| Circuit Breaker | `stateful-patterns.json` | ⚠️ Parse only |
| Idempotent | `stateful-patterns.json` | ⚠️ Parse only |
| Dead Letter | (not in fixtures) | ⚠️ IR only |
| Saga | (partial) | ⚠️ Executor exists, integration incomplete |
| Claim Check | (not in fixtures) | ⚠️ IR only |

## Test Categories

1. **Parse Tests**: Verify JSON can be deserialized
2. **Round-Trip Tests**: Verify serialize(deserialize(json)) == json
3. **Field Tests**: Verify specific fields are parsed correctly

**Important**: Round-trip tests are only run for **implemented** patterns.
Stateful patterns only get parse tests since we don't want to commit
to their exact JSON shape until runtime is implemented.

## Adding New Fixtures

When adding patterns:

1. If runtime is implemented: Add full round-trip test
2. If IR only: Add parse-only test with clear "[NOT IMPLEMENTED]" marker
3. Update this README with status
