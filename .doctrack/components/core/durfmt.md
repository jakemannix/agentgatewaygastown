---
feature: core
type: component
files:
  - crates/core/src/durfmt.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# DurFmt (Duration Formatting)

## Responsibility

Parse Go-style duration strings and format `Duration` values into human-readable strings with 3-significant-figure rounding.

## Internal Logic

### Parsing

`parse(string)` uses `go_parse_duration` to parse strings like `"5s"`, `"1m30s"`, `"100ms"`. Rejects negative durations.

### Formatting

`format(d)` produces human-readable output using `durationfmt::to_string`, with values rounded to 3 significant figures:

| Duration | Output |
|----------|--------|
| 0 | `"0s"` |
| 1ns | `"1ns"` |
| 12.345us | `"12.345us"` |
| 2.2ms | `"2.2ms"` |
| 3.3s | `"3.3s"` |
| 4m5s | `"4m5s"` |
| 5h6m7.001s | `"5h6m7.001s"` |

### Rounding Logic

`round_to_3_figs(d)`:
- <= 1ms: no rounding (preserve precision)
- <= 1s: round to nearest microsecond
- > 1s: round to nearest millisecond
- Handles overflow (rounded nanos >= 1 billion increments seconds)

## Relationships

- **Used by**: `crates/agentgateway` for logging connection durations and timeout configuration
- **Parent**: [[features/core|Core]]
