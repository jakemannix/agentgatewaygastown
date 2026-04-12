---
type: interface
implementors:
  - xds
  - agentgateway-config
consumers:
  - agentgateway-store
  - agentgateway-proxy
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/interface
  - doctrack/status/active
  - doctrack/audience/claude
---

# Interface: XDS / Config → IR Contract

## Contract
Both local configuration (YAML/JSON files) and XDS (remote control plane) produce the same Internal Representation (IR) types. The IR is stored in `Stores` and consumed at runtime by the proxy.

## IR Hierarchy
```
Bind → Listener → Route → Backend
  ↑        ↑         ↑        ↑
Policy  Policy    Policy   Policy
```

Policies attach to any level with merging semantics. Precedence resolved at runtime.

## Implementors
- [[features/agentgateway-config|Config parsing]] — local YAML/JSON → IR
- [[features/xds|XDS]] — remote proto → IR
- [[features/agentgateway-types|Types]] — `agent_xds.rs` XDS translation

## Consumers
- [[features/agentgateway-store|Store]] — holds the runtime IR
- [[features/agentgateway-proxy|Proxy]] — reads IR for routing decisions

## Related
- [[concepts/three-layer-config|Three-Layer Configuration Model]]
- [[references/imported/configuration|Configuration Architecture]]
