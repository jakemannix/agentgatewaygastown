---
feature: core
type: component
files:
  - crates/core/src/copy.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
---

# Copy (Async Bidirectional I/O)

## Responsibility

High-performance async bidirectional data copy between two I/O streams with adaptive buffer resizing, vectored writes, and metrics reporting.

## Internal Logic

### Core Function

`copy_bidirectional(downstream, upstream, stats)` splits both streams into buffered readers and writers, then runs two copy tasks concurrently via `tokio::join!`:
- `downstream_to_upstream` (send direction)
- `upstream_to_downstream` (receive direction)

Each direction handles its own shutdown when the copy completes. Errors are translated into semantic types (`BackendDisconnected`, `ClientDisconnected`).

### Traits

**`BufferedSplitter`** - Splits an I/O object into a buffered reader (`ResizeBufRead`) and a writer (`AsyncWriteBuf`). Generic impl for any `AsyncRead + AsyncWrite`. Specialized `TcpStreamSplitter` avoids the lock overhead of `tokio::io::split` by using `TcpStream::into_split()`.

**`AsyncWriteBuf`** - Like `AsyncWrite` but takes `Bytes` instead of `&[u8]`, avoiding copies via reference counting.

**`ResizeBufRead`** - Like `AsyncBufRead` but with a `resize()` method to dynamically grow the internal buffer.

### Adaptive Buffer Sizing

Inspired by Go's TLS connection buffer strategy:

| Data transferred | Buffer size | Constant |
|------------------|-------------|----------|
| 0 - 128KB | 1KB | `INITIAL_BUFFER_SIZE` |
| 128KB - 10MB | ~16KB | `LARGE_BUFFER_SIZE` (16384 - 64) |
| > 10MB | ~256KB | `JUMBO_BUFFER_SIZE` (16 * 16384 - 64) |

The -64 leaves room for H2 frame headers within TLS record boundaries.

### CopyBuf Future

Fork of Tokio's `CopyBuf` with additions:
- Supports `ResizeBufRead` for dynamic buffer growth
- Reports bytes transferred via `ConnectionResult.increment_send/recv`
- Handles partial writes by storing remaining bytes for the next poll

### Error Handling

`CopyError` enum includes proxy-specific error types:
- `Bind`, `Io`, `ShutdownError`
- `BackendDisconnected`, `ClientDisconnected` (translated from common IO error kinds)
- `SelfCall` (recursive call detection)
- `NoResolvedAddresses`, `NoPortForServices`, `NoIPForService`

`ignore_io_errors` silently absorbs `NotConnected`, `UnexpectedEof`, `ConnectionReset`, `BrokenPipe` -- these are normal TCP lifecycle events, not actionable errors.

## Relationships

- **Used by**: `crates/hbone` for HTTP/2 CONNECT tunneling, `crates/agentgateway` proxy
- **Parent**: [[features/core|Core]]
