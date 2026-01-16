# Pattern Demos

This directory contains demonstrations of various agent patterns integrated with agentgateway.

## Agents

| Agent | Framework | Pattern | Description |
|-------|-----------|---------|-------------|
| [google_adk_agent](./agents/google_adk_agent/) | Google ADK | Saga Pattern | Multi-step project setup with compensating transactions |

## Adding New Pattern Demos

Pattern demos should demonstrate:

1. **Integration with agentgateway** - How the agent framework connects to MCP tools
2. **A specific pattern** - A well-known architectural or integration pattern
3. **Practical use case** - A realistic scenario that showcases the pattern

### Directory Structure

```
pattern-demos/
└── agents/
    └── <framework>_<pattern>_agent/
        ├── __init__.py
        ├── __main__.py
        ├── agent.py
        ├── config.yaml       # AgentGateway config
        ├── pyproject.toml
        └── README.md
```

## Patterns to Explore

- **Saga Pattern** - Distributed transactions with compensating actions (implemented)
- **Circuit Breaker** - Fault tolerance for external service calls
- **Event Sourcing** - State reconstruction from event history
- **CQRS** - Command Query Responsibility Segregation
- **Scatter-Gather** - Parallel tool invocation and result aggregation
