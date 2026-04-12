---
component: celx-strings
parent_feature: celx
type: component
doctrack_version: 3.0.0
files:
  - crates/celx/src/strings.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - Strings
  - '103'
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Strings Extension Module

## Purpose

Implements the [CEL Strings extension](https://pkg.go.dev/github.com/google/cel-go/ext#Strings) from the cel-go specification. This provides standard string manipulation functions that are not part of the core CEL spec but are widely expected by policy authors.

## Provenance

Ported from [Kuadrant/wasm-shim](https://github.com/Kuadrant/wasm-shim) under Apache 2.0 license. There is an open upstream issue ([cel-rust#103](https://github.com/cel-rust/cel-rust/issues/103)) to contribute this back to the cel-rust project.

## Functions

All functions are method-style (called on a `this` string or list value):

| Function | Example | Notes |
|----------|---------|-------|
| `charAt(i)` | `"hello".charAt(1)` -> `"e"` | Returns error if index out of bounds |
| `indexOf(sub, [start])` | `"hello".indexOf("ll")` -> `2` | Returns -1 if not found; optional start offset |
| `lastIndexOf(sub, [start])` | `"hello".lastIndexOf("l")` -> `3` | Reverse search |
| `join([sep])` | `["a","b"].join(",")` -> `"a,b"` | Separator defaults to empty string |
| `lowerAscii()` | `"Hello".lowerAscii()` -> `"hello"` | ASCII only (not Unicode) |
| `upperAscii()` | `"Hello".upperAscii()` -> `"HELLO"` | ASCII only |
| `trim()` | `" hi ".trim()` -> `"hi"` | Trims whitespace |
| `replace(from, to, [n])` | `"aaa".replace("a","b",2)` -> `"bba"` | Optional count limit |
| `split(sep, [limit])` | `"a,b,c".split(",")` -> `["a","b","c"]` | Optional split limit via `splitn` |
| `substring(start, [end])` | `"hello".substring(1,3)` -> `"el"` | End defaults to string length; error if end < start |

## Implementation Notes

- Uses `Arguments` extractor for variadic argument handling (1 or 2 args for indexOf, replace, split, substring).
- Character indexing uses Rust's `chars()` iterator (Unicode code points), not byte offsets.
- `join()` requires all list elements to be strings; returns error on non-string elements.

## Related

- [[features/celx|celx Feature]] — parent feature
