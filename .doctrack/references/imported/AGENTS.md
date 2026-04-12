---
type: reference
original_path: docs/AGENTS.md
last_updated: 2026-04-12T00:00:00.000Z
status: active
tags:
  - doctrack/type/reference
  - doctrack/status/active
  - doctrack/audience/claude
---

# AGENTS.md — AI Agent Instructions (imported)

Source: `docs/AGENTS.md`

## Purpose
Provides guidance to AI agents working with the vMCP (Virtual Model Context Protocol) Tool Integration Algebra design documents. This is a meta-document — instructions for how agents should interact with the codebase.

## Key Points

- **Project type**: Research and conceptual framework, not executable code. The primary artifact is a comprehensive design document proposing a compositional algebra for AI tool integration.
- **Inspired by Apache Camel**: Maps enterprise integration patterns (EIP) onto MCP servers. The core insight is that enterprise integration and AI tool orchestration face the same fundamental problem — composing heterogeneous, stateful services without custom glue code.

## Pattern Inventory (21 total)

**16 Enterprise Integration Patterns (Camel-inspired):**
Pipeline, Content-Based Router, Splitter, Aggregator, Scatter-Gather, Enricher, Normalizer, Filter, Recipient List, Wire Tap, Dead Letter Channel, Circuit Breaker, Throttler, Idempotent Consumer, Claim Check, Saga

**5 MCP-Specific Operations:**
Tool Adapter (1:1 transforms), Schema Mediator (semantic schema transformation), Capability Router (dynamic tool selection), Semantic Deduplicator (similarity-based dedup), Confidence Aggregator (weighted source aggregation)

## Design Principles
- Declarative, not imperative — routes defined as DSL, not code
- Composable primitives — complex workflows from simple operators
- Transparent composition — composed tools indistinguishable from primitives
- Context-aware — explicit control over LLM input/output shaping (token efficiency)
- Observable — first-class tracing and debugging

## Related Notes
- [[references/imported/mcp-algebra|MCP Algebra]] — the compositional algebra itself
- [[references/imported/virtual-tools|Virtual Tools]] — implementation docs
- [[references/imported/ALIGNMENT_WITH_AGENTGATEWAY|Alignment Analysis]] — how this maps to agentgateway
