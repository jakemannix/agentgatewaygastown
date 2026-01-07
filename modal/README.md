# Modal Deployment for Agentgateway

Deploy agentgateway as a serverless container on [Modal](https://modal.com), providing a scalable, managed MCP/A2A gateway service.

## Prerequisites

1. **Modal account**: Sign up at [modal.com](https://modal.com)
2. **Modal CLI**: Install with `pip install modal`
3. **Authenticate**: Run `modal token new` to authenticate

## Quick Start

### Option 1: Build from Source (Recommended for Development)

This option builds agentgateway from the project's Dockerfile:

```bash
# Deploy to Modal
modal deploy modal/modal_app.py

# Or serve locally for testing
modal serve modal/modal_app.py
```

### Option 2: Use Pre-built Image (Faster Deployment)

This option uses a pre-built container image from the registry:

```bash
# Deploy using pre-built image
modal deploy modal/modal_app_prebuilt.py
```

## Configuration

Agentgateway is configured via Modal Secrets. Create a secret with your configuration:

```bash
# Create secret from a config file
modal secret create agentgateway-config \
    AGENTGATEWAY_CONFIG="$(cat your-config.yaml)"

# Or create secret with inline config
modal secret create agentgateway-config \
    AGENTGATEWAY_CONFIG='
binds:
- port: 3000
  listeners:
  - routes:
    - backends:
      - mcp:
          targets:
          - name: my-server
            sse:
              uri: https://my-mcp-server.example.com/sse
'
```

See `examples/basic_config.yaml` for a template configuration.

## Environment Variables

Set these via Modal Secrets:

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENTGATEWAY_CONFIG` | Full YAML configuration | Minimal default config |
| `AGENTGATEWAY_CONFIG_FILE` | Path to config file in container | None |
| `AGENTGATEWAY_IMAGE` | Container image (prebuilt only) | `ghcr.io/agentgateway/agentgateway:latest` |

## Deployment Options

### Resource Configuration

Edit the `@app.function()` decorator in the Modal app files to adjust resources:

```python
@app.function(
    cpu=2.0,           # CPU cores (0.25 to 8)
    memory=1024,       # Memory in MB (128 to 32768)
    gpu=None,          # GPU type if needed
    timeout=600,       # Max execution time in seconds
    keep_warm=1,       # Keep N containers warm (costs money)
    allow_concurrent_inputs=100,  # Concurrent requests per container
)
```

### Custom Domain

Configure a custom domain in your Modal dashboard after deployment.

### Scaling

Modal automatically scales based on incoming requests. Configure scaling behavior:

```python
@app.function(
    # Minimum containers to keep warm
    keep_warm=1,
    # Maximum concurrent requests per container
    allow_concurrent_inputs=100,
    # Container will stay alive for this long after last request
    container_idle_timeout=300,
)
```

## Development Workflow

### Local Testing

```bash
# Serve the app locally (proxied through Modal)
modal serve modal/modal_app.py

# Your endpoint will be available at:
# https://<your-workspace>--agentgateway-serve-dev.modal.run
```

### Viewing Logs

```bash
# Stream logs from your deployment
modal app logs agentgateway
```

### Updating Secrets

```bash
# Update existing secret
modal secret create agentgateway-config \
    AGENTGATEWAY_CONFIG="$(cat new-config.yaml)" \
    --force
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Modal Platform                       │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Modal Web Server                     │   │
│  │  ┌─────────────────────────────────────────┐    │   │
│  │  │         Agentgateway Container           │    │   │
│  │  │  ┌─────────────────────────────────┐    │    │   │
│  │  │  │     /app/agentgateway            │    │    │   │
│  │  │  │     (MCP/A2A Gateway)            │    │    │   │
│  │  │  └─────────────────────────────────┘    │    │   │
│  │  │           ▲           │                  │    │   │
│  │  │           │           ▼                  │    │   │
│  │  │     Port 3000    MCP Targets            │    │   │
│  │  └─────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────┘   │
│                         ▲                               │
└─────────────────────────│───────────────────────────────┘
                          │ HTTPS
                          │
              ┌───────────┴───────────┐
              │    MCP/A2A Clients    │
              │  (Claude, etc.)       │
              └───────────────────────┘
```

## Troubleshooting

### Container fails to start

1. Check logs: `modal app logs agentgateway`
2. Verify your config is valid: run `agentgateway --validate-only -f config.yaml` locally
3. Ensure your MCP targets are accessible from Modal's infrastructure

### Slow cold starts

1. Consider using `keep_warm=1` to maintain a warm container
2. Use the pre-built image option for faster container initialization

### Configuration not applying

1. Verify the secret exists: `modal secret list`
2. Check the secret value: Modal doesn't show secret values, so recreate if unsure
3. Redeploy after updating secrets: `modal deploy modal/modal_app.py`

## Cost Optimization

- Use `keep_warm=0` (default) for infrequent usage
- Adjust `cpu` and `memory` to minimum required
- Use `container_idle_timeout` to control how long containers stay alive

## Security Considerations

1. **Secrets**: Store sensitive configuration (API keys, etc.) in Modal Secrets
2. **Network**: Modal containers run in isolated environments
3. **TLS**: Modal provides automatic TLS termination
4. **CORS**: Configure appropriate CORS policies in your agentgateway config

## Examples

See the `examples/` directory for configuration templates:

- `basic_config.yaml`: Minimal configuration template

## Support

- [Agentgateway Documentation](https://agentgateway.dev/docs/)
- [Modal Documentation](https://modal.com/docs)
- [GitHub Issues](https://github.com/agentgateway/agentgateway/issues)
