# Design: Parallel DAG Executor for Pipeline Patterns

## Status
Proposed

## Context

The current `PipelineExecutor` executes steps sequentially in array order, even when steps have no inter-dependencies. The IR already supports DAG structures through `StepBinding` references, allowing any step to reference any previous step's output. A test (`test_dag_pattern_with_parallel_branches`) confirms this works correctly today.

However, we're leaving performance on the table. In a pipeline like:

```
    input
    /   \
   v     v
prefs   embed    <- could run in parallel
   \     /
    v   v
   search
```

Steps `prefs` and `embed` both only depend on the original input, yet they execute sequentially.

## Goals

1. Execute independent steps in parallel without changing the IR
2. Maintain backward compatibility - existing pipelines produce identical results
3. Provide clear error messages for invalid DAGs (cycles, missing refs)
4. Minimal changes to existing code structure

## Non-Goals

1. Changing the pipeline IR schema
2. Adding explicit `after: [...]` dependency declarations
3. Distributed execution across nodes
4. Speculative execution or cancellation

## Design

### 1. Dependency Extraction

Add a function to extract dependencies from a step's input binding:

```rust
fn extract_step_dependencies(binding: &DataBinding) -> HashSet<String> {
    match binding {
        DataBinding::Input(_) => HashSet::new(),
        DataBinding::Constant(_) => HashSet::new(),
        DataBinding::Step(sb) => {
            let mut deps = HashSet::new();
            deps.insert(sb.step_id.clone());
            deps
        }
        DataBinding::Construct(cb) => {
            cb.fields
                .values()
                .flat_map(|b| extract_step_dependencies(b))
                .collect()
        }
    }
}
```

For a step, if `input` is `None`, it depends on the previous step (sequential behavior). If `input` is `Some(binding)`, extract dependencies from the binding.

### 2. Dependency Graph Construction

Build a map from step ID to its dependencies:

```rust
struct DependencyGraph {
    /// step_id -> set of step_ids it depends on
    dependencies: HashMap<String, HashSet<String>>,
    /// step_id -> set of step_ids that depend on it
    dependents: HashMap<String, HashSet<String>>,
    /// Steps in original order (for deterministic output)
    step_order: Vec<String>,
}

impl DependencyGraph {
    fn from_pipeline(spec: &PipelineSpec) -> Result<Self, ExecutionError> {
        let mut dependencies: HashMap<String, HashSet<String>> = HashMap::new();
        let mut dependents: HashMap<String, HashSet<String>> = HashMap::new();
        let mut step_order = Vec::new();
        let mut prev_step: Option<String> = None;

        for step in &spec.steps {
            step_order.push(step.id.clone());

            let deps = if let Some(ref binding) = step.input {
                extract_step_dependencies(binding)
            } else if let Some(ref prev) = prev_step {
                // No explicit binding = depends on previous step (legacy behavior)
                let mut deps = HashSet::new();
                deps.insert(prev.clone());
                deps
            } else {
                HashSet::new()
            };

            // Validate all dependencies exist and are earlier in the pipeline
            for dep in &deps {
                if !dependencies.contains_key(dep) {
                    return Err(ExecutionError::InvalidInput(
                        format!("Step '{}' references unknown step '{}'", step.id, dep)
                    ));
                }
                dependents.entry(dep.clone()).or_default().insert(step.id.clone());
            }

            dependencies.insert(step.id.clone(), deps);
            prev_step = Some(step.id.clone());
        }

        Ok(Self { dependencies, dependents, step_order })
    }
}
```

### 3. Execution Waves via Topological Sort

Group steps into "waves" where all steps in a wave can execute in parallel:

```rust
impl DependencyGraph {
    /// Returns steps grouped into execution waves.
    /// Wave 0: steps with no dependencies
    /// Wave N: steps whose dependencies are all in waves < N
    fn compute_waves(&self) -> Result<Vec<Vec<String>>, ExecutionError> {
        let mut waves: Vec<Vec<String>> = Vec::new();
        let mut completed: HashSet<String> = HashSet::new();
        let mut remaining: HashSet<String> = self.step_order.iter().cloned().collect();

        while !remaining.is_empty() {
            // Find all steps whose dependencies are satisfied
            let ready: Vec<String> = remaining
                .iter()
                .filter(|step_id| {
                    self.dependencies
                        .get(*step_id)
                        .map(|deps| deps.is_subset(&completed))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                // No progress = cycle detected
                return Err(ExecutionError::InvalidInput(
                    format!("Cycle detected in pipeline. Remaining steps: {:?}", remaining)
                ));
            }

            for step_id in &ready {
                remaining.remove(step_id);
                completed.insert(step_id.clone());
            }

            waves.push(ready);
        }

        Ok(waves)
    }
}
```

### 4. Parallel Execution

Replace the sequential `for` loop with wave-based parallel execution:

```rust
impl PipelineExecutor {
    pub async fn execute(
        spec: &PipelineSpec,
        input: Value,
        ctx: &ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<Value, ExecutionError> {
        // Build dependency graph
        let graph = DependencyGraph::from_pipeline(spec)?;
        let waves = graph.compute_waves()?;

        // Index steps by ID for lookup
        let steps_by_id: HashMap<&str, &PipelineStep> = spec
            .steps
            .iter()
            .map(|s| (s.id.as_str(), s))
            .collect();

        // Execute waves
        for wave in waves {
            let futures: Vec<_> = wave
                .iter()
                .map(|step_id| {
                    let step = steps_by_id[step_id.as_str()];
                    Self::execute_step(step, &input, ctx, executor)
                })
                .collect();

            // Execute all steps in this wave concurrently
            let results = futures::future::join_all(futures).await;

            // Check for errors and store results
            for (step_id, result) in wave.iter().zip(results) {
                let value = result?;
                ctx.store_step_result(step_id, value).await;
            }
        }

        // Return the last step's result (maintains pipeline semantics)
        let last_step_id = &spec.steps.last()
            .ok_or_else(|| ExecutionError::InvalidInput("Empty pipeline".into()))?
            .id;

        ctx.get_step_result(last_step_id)
            .await
            .ok_or_else(|| ExecutionError::InvalidInput("Last step produced no result".into()))
    }

    async fn execute_step(
        step: &PipelineStep,
        input: &Value,
        ctx: &ExecutionContext,
        executor: &CompositionExecutor,
    ) -> Result<Value, ExecutionError> {
        // Resolve input binding
        let step_input = if let Some(ref binding) = step.input {
            Self::resolve_binding(binding, input, ctx).await?
        } else {
            // This shouldn't happen in wave-based execution since we
            // convert None -> dependency on previous step in graph building
            input.clone()
        };

        // Execute the operation
        match &step.operation {
            StepOperation::Tool(tc) => {
                executor.execute_tool(&tc.name, step_input, ctx).await
            }
            StepOperation::Pattern(pattern) => {
                let child_ctx = ctx.child(step_input.clone());
                executor.execute_pattern(pattern, step_input, &child_ctx).await
            }
            StepOperation::Agent(ac) => {
                Err(ExecutionError::NotImplemented(format!(
                    "agent-as-tool execution not yet implemented: agent={}, skill={:?}",
                    ac.name, ac.skill
                )))
            }
        }
    }
}
```

### 5. Error Handling

| Scenario | Behavior |
|----------|----------|
| Unknown step reference | Error at graph construction: `"Step 'X' references unknown step 'Y'"` |
| Cycle detected | Error at wave computation: `"Cycle detected in pipeline"` |
| Step execution fails | Wave execution stops, error propagates up |
| Multiple failures in wave | First error wins (join_all semantics), others may still run |

**Future consideration**: Add a `fail_fast: bool` option to control whether to cancel sibling steps on first failure.

### 6. Backward Compatibility

The key insight is that `input: None` means "use previous step's output", which we translate to an explicit dependency on the previous step. This means:

- Existing pipelines with no explicit bindings execute sequentially (same as today)
- Pipelines with explicit `DataBinding::Input` bindings on independent steps now run in parallel
- Results are identical; only execution order changes

## Example

```json
{
  "pipeline": {
    "steps": [
      { "id": "a", "input": { "path": "$.x" }, "operation": { "tool": { "name": "tool_a" } } },
      { "id": "b", "input": { "path": "$.y" }, "operation": { "tool": { "name": "tool_b" } } },
      { "id": "c", "input": { "construct": {
          "from_a": { "step": { "stepId": "a", "path": "$" } },
          "from_b": { "step": { "stepId": "b", "path": "$" } }
        }}, "operation": { "tool": { "name": "tool_c" } } },
      { "id": "d", "input": { "step": { "stepId": "c", "path": "$" } }, "operation": { "tool": { "name": "tool_d" } } }
    ]
  }
}
```

**Dependency graph:**
```
a: {}
b: {}
c: {a, b}
d: {c}
```

**Execution waves:**
```
Wave 0: [a, b]    <- parallel
Wave 1: [c]
Wave 2: [d]
```

## Testing Strategy

1. **Existing tests pass unchanged** - Sequential pipelines produce same results
2. **DAG test verifies correctness** - `test_dag_pattern_with_parallel_branches` already exists
3. **Parallelism verification** - Add test with artificial delays to prove parallel execution:
   ```rust
   #[tokio::test]
   async fn test_parallel_execution_timing() {
       // Two steps that each take 100ms
       // Sequential: ~200ms
       // Parallel: ~100ms
       let start = Instant::now();
       // ... execute pipeline with two independent steps ...
       let elapsed = start.elapsed();
       assert!(elapsed < Duration::from_millis(150), "Should run in parallel");
   }
   ```
4. **Cycle detection** - Test that cycles produce clear error
5. **Missing reference** - Test that invalid step references error at construction

## Implementation Plan

1. **Phase 1**: Add `DependencyGraph` and `compute_waves()` as internal helpers
2. **Phase 2**: Add parallel execution path behind a feature flag
3. **Phase 3**: Run both paths in tests, verify identical results
4. **Phase 4**: Remove feature flag, parallel becomes default

## Future Extensions

1. **Execution metrics**: Track per-step and per-wave timing for observability
2. **Cancellation**: Cancel pending steps when one fails (requires `select!` instead of `join_all`)
3. **Resource limits**: Cap maximum concurrent steps per wave
4. **Streaming**: Allow steps to produce/consume streams for incremental processing

## Appendix: Full Dependency Extraction

Handle nested patterns (scatter-gather inside pipeline step):

```rust
fn extract_step_dependencies(binding: &DataBinding) -> HashSet<String> {
    fn recurse(binding: &DataBinding, deps: &mut HashSet<String>) {
        match binding {
            DataBinding::Input(_) | DataBinding::Constant(_) => {}
            DataBinding::Step(sb) => {
                deps.insert(sb.step_id.clone());
            }
            DataBinding::Construct(cb) => {
                for field_binding in cb.fields.values() {
                    recurse(field_binding, deps);
                }
            }
        }
    }

    let mut deps = HashSet::new();
    recurse(binding, &mut deps);
    deps
}
```
