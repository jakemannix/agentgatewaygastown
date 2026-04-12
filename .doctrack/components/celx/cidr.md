---
component: celx-cidr
parent_feature: celx
type: component
doctrack_version: 3.0.0
files:
  - crates/celx/src/cidr.rs
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/component
  - doctrack/status/active
  - doctrack/audience/claude
  - kubernetes-cidr-library
  - kubernetes-ip-address-library
---

# CIDR/IP Module

## Purpose

Implements Kubernetes-compatible CIDR and IP address functions for CEL expressions. Used in authorization policies to match client IPs against allowed/denied network ranges.

Based on:
- [Kubernetes CEL CIDR library](https://kubernetes.io/docs/reference/using-api/cel/#kubernetes-cidr-library)
- [Kubernetes CEL IP address library](https://kubernetes.io/docs/reference/using-api/cel/#kubernetes-ip-address-library)

## Opaque Types

Two custom opaque types are registered:

| Type | CEL name | Rust wrapper | Underlying |
|------|----------|-------------|------------|
| `Cidr` | `"cidr"` | `Cidr(ipnet::IpNet)` | `ipnet::IpNet` |
| `IP` | `"ip"` | `IP(net::IpAddr)` | `std::net::IpAddr` |

Both support IPv4 and IPv6.

## Function Registration

The `ip()` function is notable because it is **overloaded** — it acts as both a free function and a method:

```rust
ctx.add_function("ip", split_this(wrap1(Cidr::ip), wrapnew(IP::parse)));
```

- `ip("1.2.3.4")` — free function, parses a string into an `IP` value
- `cidr("10.0.0.0/8").ip()` — method on `Cidr`, extracts the address component

The `split_this` helper from [[components/celx/helpers|the helpers module]] dispatches based on whether `this` is set.

## Example CEL Expressions

```cel
// Authorization: allow only from internal network
cidr("10.0.0.0/8").containsIP(request.source_ip)

// Check if client is on a private network
ip(request.source_ip).isGlobalUnicast() == false

// IPv6 link-local check
ip(request.source_ip).isLinkLocalUnicast()

// CIDR containment
cidr("10.0.0.0/8").containsCIDR(cidr("10.1.0.0/16"))
```

## Related

- [[features/celx|celx Feature]] — parent feature
- [[components/celx/helpers|Helpers Module]] — `wrap1`, `wrap2`, `wrapnew`, `split_this`, `impl_opaque!`
