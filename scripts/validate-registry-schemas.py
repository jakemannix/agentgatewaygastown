#!/usr/bin/env python3
"""
Registry Schema Validator

Validates that virtual tool compositions in a registry JSON file have
consistent output schemas. This is a simple lint-style check that catches
common mistakes like:

1. Scatter-gather with heterogeneous output types and flatten aggregation
2. Missing outputSchema declarations
3. Tool references that don't exist
4. outputSchema/outputTransform mismatches (e.g., schema expects object but transform produces array)
5. Scatter-gather missing 'wrap' op when outputSchema expects an object

This is NOT a full type checker - it's a heuristic validator that flags
likely mistakes for human review.

Usage:
    python scripts/validate-registry-schemas.py examples/research-assistant-demo/gateway-configs/research_registry.json
"""

import json
import sys
from pathlib import Path
from typing import Any


class RegistryValidator:
    def __init__(self, registry: dict):
        self.registry = registry
        self.errors: list[str] = []
        self.warnings: list[str] = []

        # Build lookup tables
        self.schemas = {s["name"]: s for s in registry.get("schemas", [])}
        self.servers = {s["name"]: s for s in registry.get("servers", [])}
        self.tools = {t["name"]: t for t in registry.get("tools", [])}

        # Track which tools provide which output schemas
        self.tool_output_schemas: dict[str, str | None] = {}
        self._resolve_output_schemas()

    def _resolve_schema_ref(self, ref: str) -> dict | None:
        """Resolve a $ref to the actual schema definition."""
        if not ref:
            return None
        # Handle "#/schemas/Name" format
        if ref.startswith("#/schemas/"):
            schema_name = ref[len("#/schemas/"):]
            return self.schemas.get(schema_name, {}).get("schema")
        # Handle "#Name" or "#Name:Version" format
        if ref.startswith("#"):
            schema_name = ref[1:].split(":")[0]
            # Try with and without /schemas/ prefix
            if schema_name in self.schemas:
                return self.schemas[schema_name].get("schema")
            # Maybe it's a direct path
            parts = schema_name.split("/")
            if len(parts) >= 2 and parts[0] == "schemas":
                return self.schemas.get(parts[1], {}).get("schema")
        return None

    def _get_schema_type(self, output_schema: dict | None) -> str | None:
        """Get the root type of a schema (object, array, etc.)."""
        if not output_schema:
            return None
        if isinstance(output_schema, dict):
            if "$ref" in output_schema:
                resolved = self._resolve_schema_ref(output_schema["$ref"])
                if resolved:
                    return resolved.get("type")
            return output_schema.get("type")
        return None

    def _get_schema_properties(self, output_schema: dict | None) -> set[str]:
        """Get the property names from an object schema."""
        if not output_schema:
            return set()
        if isinstance(output_schema, dict):
            if "$ref" in output_schema:
                resolved = self._resolve_schema_ref(output_schema["$ref"])
                if resolved:
                    return set(resolved.get("properties", {}).keys())
            return set(output_schema.get("properties", {}).keys())
        return set()

    def _resolve_output_schemas(self):
        """Build a map of tool name -> output schema name."""
        for name, tool in self.tools.items():
            output_schema = tool.get("outputSchema")
            if output_schema:
                if isinstance(output_schema, dict) and "$ref" in output_schema:
                    # Parse "#SchemaName:Version" or "#/schemas/Name" format
                    ref = output_schema["$ref"]
                    if ref.startswith("#/schemas/"):
                        schema_name = ref[len("#/schemas/"):]
                        self.tool_output_schemas[name] = schema_name
                    elif ref.startswith("#"):
                        schema_name = ref[1:].split(":")[0]
                        self.tool_output_schemas[name] = schema_name
                    else:
                        self.tool_output_schemas[name] = ref
                else:
                    # Inline schema - use tool name as identifier
                    self.tool_output_schemas[name] = f"inline:{name}"
            else:
                self.tool_output_schemas[name] = None

    def _get_tool_output_schema(self, tool_name: str) -> str | None:
        """Get the output schema for a tool, resolving through source tools."""
        if tool_name in self.tool_output_schemas:
            return self.tool_output_schemas[tool_name]

        # Check if it's a source tool reference
        tool = self.tools.get(tool_name)
        if tool and "source" in tool:
            # Source tool - check if it has outputTransform
            if "outputTransform" in tool:
                # Has transform, so output differs from source
                return self.tool_output_schemas.get(tool_name)
            # No transform - output matches source (but we don't know source schema)
            return None

        return None

    def validate(self) -> bool:
        """Run all validations."""
        self._validate_tool_references()
        self._validate_scatter_gather_schemas()
        self._validate_pipeline_schemas()
        self._validate_output_schema_consistency()
        return len(self.errors) == 0

    def _validate_tool_references(self):
        """Check that all tool references exist."""
        for name, tool in self.tools.items():
            spec = tool.get("spec")
            if not spec:
                continue

            self._check_tool_refs_in_spec(spec, f"tool '{name}'")

    def _check_tool_refs_in_spec(self, spec: dict, context: str):
        """Recursively check tool references in a spec."""
        if "scatterGather" in spec:
            sg = spec["scatterGather"]
            for i, target in enumerate(sg.get("targets", [])):
                if "tool" in target:
                    tool_ref = target["tool"]
                    if tool_ref not in self.tools:
                        # Check if it's a server:tool reference
                        server = target.get("server")
                        if not server:
                            self.errors.append(
                                f"{context}: scatter-gather target[{i}] references unknown tool '{tool_ref}'"
                            )
                elif "pattern" in target:
                    self._check_tool_refs_in_spec(target["pattern"], f"{context} target[{i}]")

        if "pipeline" in spec:
            pipeline = spec["pipeline"]
            for step in pipeline.get("steps", []):
                step_id = step.get("id", "?")
                op = step.get("operation", {})
                if "tool" in op:
                    tool_info = op["tool"]
                    tool_name = tool_info.get("name") if isinstance(tool_info, dict) else tool_info
                    if tool_name not in self.tools:
                        server = tool_info.get("server") if isinstance(tool_info, dict) else None
                        if not server:
                            self.errors.append(
                                f"{context}: pipeline step '{step_id}' references unknown tool '{tool_name}'"
                            )
                elif "pattern" in op:
                    self._check_tool_refs_in_spec(op["pattern"], f"{context} step '{step_id}'")

    def _validate_scatter_gather_schemas(self):
        """Check scatter-gather compositions for schema consistency."""
        for name, tool in self.tools.items():
            spec = tool.get("spec")
            if not spec:
                continue

            self._check_scatter_gather_in_spec(spec, f"tool '{name}'")

    def _check_scatter_gather_in_spec(self, spec: dict, context: str):
        """Check a scatter-gather spec for schema consistency."""
        if "scatterGather" in spec:
            sg = spec["scatterGather"]
            targets = sg.get("targets", [])
            aggregation = sg.get("aggregation", {})
            ops = aggregation.get("ops", [])

            # Check if flatten is used
            has_flatten = any("flatten" in op for op in ops)

            if has_flatten and len(targets) > 1:
                # Collect output schemas for all targets
                target_schemas = []
                for i, target in enumerate(targets):
                    if "tool" in target:
                        tool_name = target["tool"]
                        schema = self._get_tool_output_schema(tool_name)
                        target_schemas.append((i, tool_name, schema))
                    elif "pattern" in target:
                        # Nested pattern - we can't easily determine output schema
                        target_schemas.append((i, f"pattern[{i}]", "nested_pattern"))

                # Check for heterogeneous schemas
                known_schemas = [s for _, _, s in target_schemas if s is not None]
                unique_schemas = set(known_schemas)

                if len(unique_schemas) > 1:
                    schema_list = ", ".join(
                        f"{name}={schema}" for i, name, schema in target_schemas if schema
                    )
                    self.warnings.append(
                        f"{context}: scatter-gather with 'flatten' has heterogeneous output schemas: [{schema_list}]. "
                        f"Consider using a pipeline with 'construct' to properly structure different result types."
                    )

            # Recursively check nested patterns
            for i, target in enumerate(targets):
                if "pattern" in target:
                    self._check_scatter_gather_in_spec(target["pattern"], f"{context} target[{i}]")

        if "pipeline" in spec:
            pipeline = spec["pipeline"]
            for step in pipeline.get("steps", []):
                step_id = step.get("id", "?")
                op = step.get("operation", {})
                if "pattern" in op:
                    self._check_scatter_gather_in_spec(op["pattern"], f"{context} step '{step_id}'")

    def _validate_pipeline_schemas(self):
        """Check pipeline compositions for potential issues."""
        for name, tool in self.tools.items():
            spec = tool.get("spec")
            if not spec or "pipeline" not in spec:
                continue

            pipeline = spec["pipeline"]
            output = pipeline.get("output")

            # Check if tool has outputSchema but no output construct
            if tool.get("outputSchema") and not output:
                self.warnings.append(
                    f"tool '{name}': has outputSchema but pipeline has no output construct"
                )

    def _validate_output_schema_consistency(self):
        """Check that outputSchema matches what outputTransform/aggregation produces."""
        for name, tool in self.tools.items():
            output_schema = tool.get("outputSchema")
            if not output_schema:
                continue

            schema_type = self._get_schema_type(output_schema)
            schema_properties = self._get_schema_properties(output_schema)

            # Check outputTransform consistency
            output_transform = tool.get("outputTransform")
            if output_transform:
                self._check_transform_matches_schema(
                    name, output_transform, schema_type, schema_properties
                )

            # Check scatter-gather aggregation consistency
            spec = tool.get("spec")
            if spec and "scatterGather" in spec:
                self._check_scatter_gather_matches_schema(
                    name, spec["scatterGather"], schema_type, schema_properties
                )

    def _check_transform_matches_schema(
        self,
        tool_name: str,
        transform: dict,
        schema_type: str | None,
        schema_properties: set[str],
    ):
        """Check that outputTransform produces output matching outputSchema."""
        mappings = transform.get("mappings", {})
        transform_fields = set(mappings.keys())

        if schema_type == "object" and schema_properties:
            # Check that transform produces the expected fields
            missing_fields = schema_properties - transform_fields
            extra_fields = transform_fields - schema_properties

            if missing_fields:
                self.warnings.append(
                    f"tool '{tool_name}': outputTransform missing fields from outputSchema: {missing_fields}"
                )
            if extra_fields:
                self.warnings.append(
                    f"tool '{tool_name}': outputTransform produces fields not in outputSchema: {extra_fields}"
                )

        elif schema_type == "array":
            # Transform produces an object but schema expects array
            if mappings:
                self.errors.append(
                    f"tool '{tool_name}': outputSchema expects array but outputTransform produces object with fields: {transform_fields}"
                )

    def _check_scatter_gather_matches_schema(
        self,
        tool_name: str,
        sg_spec: dict,
        schema_type: str | None,
        schema_properties: set[str],
    ):
        """Check that scatter-gather aggregation produces output matching outputSchema."""
        aggregation = sg_spec.get("aggregation", {})
        ops = aggregation.get("ops", [])

        # Check if there's a wrap op
        wrap_op = None
        for op in ops:
            if isinstance(op, dict) and "wrap" in op:
                wrap_op = op["wrap"]
                break

        if schema_type == "object":
            if not wrap_op:
                # Schema expects object but no wrap - will produce flat array
                self.errors.append(
                    f"tool '{tool_name}': outputSchema expects object but scatter-gather has no 'wrap' aggregation op. "
                    f"Add {{\"wrap\": {{\"field\": \"<fieldname>\"}}}} to aggregation.ops"
                )
            elif schema_properties:
                # Check that wrap field matches a property in the schema
                wrap_field = wrap_op.get("field")
                if wrap_field and wrap_field not in schema_properties:
                    self.errors.append(
                        f"tool '{tool_name}': wrap field '{wrap_field}' not in outputSchema properties: {schema_properties}"
                    )

        elif schema_type == "array":
            if wrap_op:
                # Schema expects array but wrap will produce object
                self.errors.append(
                    f"tool '{tool_name}': outputSchema expects array but scatter-gather has 'wrap' op that produces object"
                )

    def report(self) -> str:
        """Generate a validation report."""
        lines = []

        if self.errors:
            lines.append(f"ERRORS ({len(self.errors)}):")
            for err in self.errors:
                lines.append(f"  ✗ {err}")
            lines.append("")

        if self.warnings:
            lines.append(f"WARNINGS ({len(self.warnings)}):")
            for warn in self.warnings:
                lines.append(f"  ⚠ {warn}")
            lines.append("")

        if not self.errors and not self.warnings:
            lines.append("✓ No schema issues found")

        return "\n".join(lines)


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <registry.json> [--strict]")
        print()
        print("Options:")
        print("  --strict    Treat warnings as errors")
        sys.exit(1)

    registry_path = Path(sys.argv[1])
    strict = "--strict" in sys.argv

    if not registry_path.exists():
        print(f"Error: File not found: {registry_path}")
        sys.exit(1)

    with open(registry_path) as f:
        registry = json.load(f)

    validator = RegistryValidator(registry)
    valid = validator.validate()

    print(f"Validating: {registry_path}")
    print()
    print(validator.report())

    if not valid:
        sys.exit(1)
    elif strict and validator.warnings:
        print("Failing due to --strict mode")
        sys.exit(1)


if __name__ == "__main__":
    main()
