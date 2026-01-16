# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

- **Name**: vMCP (Virtual Model Context Protocol) - Tool Integration Algebra
- **Type**: Research & Conceptual Framework / Design Document
- **Primary Language**: Markdown (Documentation)
- **Framework**: Architecture & Systems Design (inspired by Apache Camel)
- **Description**: A comprehensive proposal for a compositional algebra to solve the tool integration problem in AI agent workflows. Outlines how MCP (Model Context Protocol) servers can be composed using patterns derived from enterprise integration (Apache Camel), enabling declarative, reusable, and maintainable multi-tool orchestration without imperative glue code.

## Architecture & Structure

This repository contains a single, comprehensive design document that proposes a layered architecture for AI tool composition:

1. **Agent Layer** - AI agents (coding, research, data, service agents) that discover and invoke tools
2. **MCP Registry** - Central catalog for tool discovery, schema introspection, and health monitoring
3. **vMCP Runtime** - The core composition engine implementing the Tool Integration Algebra
4. **MCP Server Ecosystem** - Heterogeneous primitive MCP servers and composed vMCP servers

The document articulates:
- **System Architecture** - Component relationships and interaction flows
- **Enterprise Integration Patterns** - 16 foundational composition primitives adapted to MCP
- **MCP-Specific Operations** - Advanced patterns unique to AI/LLM tool contexts
- **Declarative DSL Concepts** - Pseudo-code examples of a composition language
- **Core Problem & Vision** - Why tool composition matters for LLM context efficiency

## Technology Stack

### Architecture & Patterns
- **Enterprise Integration Patterns** (Apache Camel inspired)
  - Pipeline, Router, Splitter, Aggregator, Scatter-Gather
  - Enricher, Normalizer, Filter, Recipient List
  - Wire Tap, Dead Letter Channel, Circuit Breaker
  - Throttler, Idempotent Consumer, Claim Check, Saga

- **MCP-Specific Patterns**
  - Tool Adapter (1:1 transforms)
  - Schema Mediator (semantic schema transformation)
  - Capability Router (dynamic tool selection)
  - Semantic Deduplicator (similarity-based dedup)
  - Confidence Aggregator (weighted source aggregation)

### Core Concepts
- **Type System** - Functional composition with type signatures
- **Composition Model** - Tools as first-class composable primitives
- **Context Optimization** - Token-aware design for LLM efficiency
- **Observability** - Built-in tracing and observability patterns
- **Error Handling** - Retry policies, fallbacks, circuit breakers

### Design Principles
- **Declarative, not imperative** - Routes defined as DSL, not code
- **Composable primitives** - Complex workflows from simple operators
- **Transparent composition** - Composed tools indistinguishable from primitives
- **Context-aware** - Explicit control over LLM input/output shaping
- **Observable** - First-class support for debugging and auditing

## Development Guidelines

### Code Style & Conventions

This is a **research and design document**, not executable code. The document uses:

- **Mermaid diagrams** - Visual representation of architecture and data flows
- **Type signatures** - Functional notation (e.g., `(A -> B) × [Tool] -> Tool`)
- **Pseudo-code** - Conceptual DSL examples (not executable)
- **Comparison tables** - Side-by-side Apache Camel ↔ MCP patterns
- **Concrete examples** - Real-world use cases for each pattern

When extending this document:
- Maintain consistent notation (type signatures, pseudocode style)
- Include visual Mermaid diagrams for complex patterns
- Provide concrete, relatable examples for each abstraction
- Include problem statement → solution → benefits for new patterns
- Keep the Apache Camel analogy thread consistent (it grounds unfamiliar concepts)

### Testing Strategy

This is a **conceptual framework document**. Testing would occur at implementation time:

- **Pattern validation** - Each pattern should be implementable without ad-hoc extensions
- **Composition algebra verification** - Patterns should compose correctly (e.g., `pipeline ∘ scatter_gather`)
- **Type safety** - Composition type signatures must remain sound
- **Context efficiency** - Composed tools must not bloat LLM context
- **Real-world workflows** - Each pattern should solve a genuine use case

### Git Workflow

- Treat this as a living design document
- Changes should be thematic (e.g., "Add schema_mediate pattern", "Expand error handling patterns")
- Commit messages should reference the patterns or concepts being added/modified
- Use conventional prefixes: `docs:`, `design:`, `refactor:` (no `feat:` or `fix:`)

### Documentation Standards

- Keep document length reasonable (one comprehensive file, not fragmented)
- Use clear section hierarchy (H1 → H2 → H3 only)
- Every pattern must include:
  - **Camel equivalent** (grounds readers familiar with Camel)
  - **MCP algebra pseudocode** (shows MCP application)
  - **Concrete example** (relatable use case)
  - **Mermaid diagram** (visual representation)
  - **Key insight** (why this matters)
- Include tables for quick reference (patterns, type signatures, etc.)
- Frontload the problem statement—readers need context before solutions

## AI Agent Instructions

### Project Context

- **Working Style**: Research and conceptual design with practical grounding
- **Focus Areas**:
  - Solving the tool composition problem for AI agents
  - Token efficiency and context management
  - Composability without reimplementation
  - Real-world applicability

- **Common Tasks**:
  - Extending the pattern catalog with new composition primitives
  - Improving clarity of existing patterns
  - Adding concrete use cases or refining examples
  - Refactoring for consistency and coherence
  - Creating comparison tables or summary matrices

### Content Guidelines

When working with this document, focus on:

1. **Clarity over completeness** - Readers should understand the problem and solution, not get lost in implementation details
2. **Grounded abstractions** - Every pattern should solve a real problem agents face
3. **Consistent notation** - Maintain the type signature notation, pseudocode style, and Apache Camel parallels
4. **Visual communication** - Use Mermaid diagrams extensively; they clarify complex architectures
5. **Concrete examples** - Abstract patterns are hard to reason about; examples make them concrete

### Best Practices for This Project

1. **Type Signatures** - Use functional notation consistently:
   - `Tool` = composition primitive
   - `(A -> B)` = transformation or predicate
   - `[Tool]` = list of tools
   - `×` = product (combining multiple)
   - `|` = coproduct (alternatives)

2. **Pseudocode DSL** - Keep pseudo-code concise but readable:
   - Use declarative style (describe what, not how)
   - Assume familiar syntax (Python-like indentation, `->` for returns)
   - Avoid language-specific keywords; use concepts

3. **Examples** - Every pattern should have:
   - **Problem statement** - What the pattern solves
   - **Pseudocode solution** - How it composes
   - **Concrete use case** - Real-world application
   - **Visual diagram** - Data flow or architecture

4. **Diagrams** - Use Mermaid for:
   - **Architecture diagrams** (`graph LR`, `graph TD`)
   - **Data flow diagrams** (node → node with labeled edges)
   - **State machines** (if error handling or circuits are involved)
   - **Sequence diagrams** (for interaction patterns)

5. **Comparisons** - When introducing new patterns:
   - Compare to existing patterns (how is it different?)
   - Compare to Apache Camel equivalents (ground unfamiliar readers)
   - Explain when to use it (vs. other approaches)

## Quick Reference

### Pattern Inventory (16 Enterprise Integration + 5 MCP-Specific)

**Enterprise Integration Patterns (Camel-inspired):**
1. Pipeline - Sequential tool chaining
2. Content-Based Router - Dispatch by input
3. Splitter - Process collection items independently
4. Aggregator - Collect and combine results
5. Scatter-Gather - Parallel multicast + aggregate
6. Enricher - Augment data from other tools
7. Normalizer - Convert to common schema
8. Filter - Conditionally pass/block
9. Recipient List - Dynamic routing
10. Wire Tap - Observability/side channels
11. Dead Letter Channel - Error handling
12. Circuit Breaker - Protect against cascades
13. Throttler - Rate limiting
14. Idempotent Consumer - Deduplication
15. Claim Check - Large payload handling
16. Saga - Multi-step with compensation

**MCP-Specific Operations:**
17. Tool Adapter - 1:1 tool transformation
18. Schema Mediator - Transform between schemas
19. Capability Router - Match by tool capabilities
20. Semantic Deduplicator - Similarity-based dedup
21. Confidence Aggregator - Weight by source reliability

### Key Concepts

- **Composition** - Tools compose like functions without reimplementation
- **Context efficiency** - Explicit control over LLM input/output shape
- **Declarative DSL** - Define workflows, not glue code
- **First-class patterns** - Common integration problems are primitives
- **Transparent abstraction** - Composed tools look identical to primitives
- **Observability** - Tracing and debugging built-in

### Document Structure

1. **Problem** (lines 1-27) - Context and motivation
2. **Vision** (lines 28-34) - The proposed solution
3. **Core Analogy** (lines 35-45) - Apache Camel parallel
4. **Value Proposition** (lines 49-63) - Why this matters
5. **System Architecture** (lines 64-198) - Components and interaction
6. **Pattern Catalog** (lines 199-1304) - 16 enterprise patterns + examples
7. **Compositional Algebra DSL** (lines 1305-1370) - Pseudo-code example
8. **MCP-Specific Operations** (lines 1374-1824) - 5 advanced patterns
9. **Summary** (lines 1828-1850) - Quick reference table

## Notes & Context

### Why Apache Camel?

Apache Camel is a battle-tested enterprise integration framework with 300+ endpoint adapters and rich composition patterns. By mapping MCP tool integration onto Camel's proven algebra, we inherit:
- Decades of experience solving integration problems
- Well-understood semantics for each pattern
- A common vocabulary (Camel users immediately understand the analogy)
- Confidence that this approach scales to complex workflows

The key insight: **Enterprise integration and AI tool orchestration face the same fundamental problem—composing heterogeneous, stateful services without writing custom glue code for every workflow.**

### Core Tension: Token Efficiency

The document repeatedly emphasizes a critical constraint agents face: **every byte in the LLM context counts**. Tool descriptions, schemas, and outputs all consume tokens. This motivates:

- **Tool Adapter** - Curate descriptions and output schemas
- **Schema Mediator** - Transform incompatible formats without bloat
- **Normalizer** - Unify heterogeneous outputs to minimal schemas
- **Filter** - Remove noise before returning to agent
- **Confidence Aggregator** - Return weighted results, not all possibilities

An agent developer who cedes this control to upstream tool authors wastes context budget. Composition patterns let them reclaim it.

### Common Extension Points

When extending this document, consider:

1. **New enterprise patterns** - Are there Camel patterns not covered?
2. **Domain-specific patterns** - Are there agent-specific patterns beyond Camel?
3. **Error handling patterns** - Dead Letter Channel covers basics; are there others?
4. **Optimization patterns** - Caching, batching, request deduplication?
5. **Security patterns** - Rate limiting, access control, audit logging?
6. **Monitoring patterns** - Observability beyond Wire Tap?

### Implementation Considerations (Future)

This document is a **specification**, not an implementation. A real vMCP runtime would need:

- **DSL Parser** - Parse composition definitions into execution DAGs
- **Type Checker** - Validate composition type safety
- **Query Optimizer** - Reorder operations for efficiency (early filtering, caching)
- **Executor** - Manage tool invocations, data flow, error handling
- **Distributed Tracer** - Observability across composed workflows
- **Result Cache** - Memoization for idempotent operations
- **Registry Integration** - Discover and monitor MCP servers

The document doesn't prescribe implementation details, leaving room for multiple approaches (declarative DSL, imperative builder API, configuration-driven, etc.).

### Audience

This document is intended for:
- **System architects** designing AI agent platforms
- **Agent developers** frustrated with tool integration boilerplate
- **MCP server authors** thinking about composability
- **LLM framework maintainers** considering tool orchestration
- **Enterprise integration veterans** curious about MCP applications

It assumes familiarity with:
- Model Context Protocol (MCP) concepts
- Basic functional programming notation
- Enterprise integration patterns (optionally; Apache Camel comparison provides context)
- AI agent workflows and LLM context constraints

## Wibey Commands Most Useful Here

- `/cat` - View full document content
- `/search` or `Ctrl+F` - Navigate patterns by keyword
- `!wc` - Check document length
- `!grep "pattern_name"` - Find specific pattern in document
- Documentation editing - Use Edit tool for pattern updates
