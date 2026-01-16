# MCP Notification Service

A Python MCP server demonstrating async notification patterns with queuing, retry logic, and dead-letter handling.

## Features

- **Multiple Channels**: Email (mock), Slack (mock), In-App notifications
- **Async Delivery Queue**: Notifications are queued and processed asynchronously
- **Retry Logic**: Failed deliveries are automatically retried with exponential backoff
- **Dead-Letter Queue**: Permanently failed notifications are moved to a DLQ for inspection
- **Configurable Failure Rates**: Test retry and dead-letter patterns by adjusting mock failure rates

## Tools

### `send_notification`

Send a notification to a user via the specified channel.

```json
{
  "user_id": "user123",
  "subject": "Welcome!",
  "body": "Thanks for signing up.",
  "channel": "email",
  "priority": "normal",
  "metadata": {"template": "welcome"}
}
```

**Channels**: `email`, `slack`, `in_app`
**Priorities**: `low`, `normal`, `high`, `urgent`

### `get_notifications`

Retrieve notifications for a user with optional filters.

```json
{
  "user_id": "user123",
  "channel": "email",
  "status": "delivered",
  "limit": 50
}
```

**Statuses**: `pending`, `queued`, `delivered`, `failed`, `read`

### `mark_read`

Mark a notification as read.

```json
{
  "notification_id": "abc-123-def"
}
```

### `get_dead_letter_queue`

View notifications that failed delivery after all retry attempts.

```json
{
  "limit": 50
}
```

### `retry_dead_letter`

Retry a failed notification from the dead-letter queue.

```json
{
  "notification_id": "abc-123-def"
}
```

### `configure_failure_rates`

Adjust mock failure rates for testing. Useful for demonstrating retry and dead-letter patterns.

```json
{
  "email": 0.5,
  "slack": 0.1,
  "in_app": 0.0
}
```

## Running the Server

### Standalone (stdio)

```bash
cd examples/pattern-demos/mcp-tools/notification_service
python server.py
```

### With SSE transport

```bash
python server.py --sse
```

### With Agent Gateway

Add to your gateway config:

```yaml
listeners:
  - address: "localhost:3000"

targets:
  notification:
    mcp_server:
      command: python
      args:
        - examples/pattern-demos/mcp-tools/notification_service/server.py
```

## Demo: Dead-Letter Pattern

1. Configure high failure rate:
   ```json
   {"email": 0.9}  // 90% failure rate
   ```

2. Send notifications:
   ```json
   {
     "user_id": "demo_user",
     "subject": "Test",
     "body": "This will likely fail",
     "channel": "email"
   }
   ```

3. After retries exhaust, check the dead-letter queue:
   ```json
   {"limit": 10}
   ```

4. Investigate and retry:
   ```json
   {"notification_id": "..."}
   ```

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌───────────────┐
│  send_notif.    │────▶│  Delivery Queue  │────▶│  Processor    │
└─────────────────┘     └──────────────────┘     └───────┬───────┘
                                                         │
                        ┌────────────────────────────────┼────────────────────────────────┐
                        │                                ▼                                │
                        │         ┌──────────────────────────────────────┐               │
                        │         │           Channel Handlers           │               │
                        │         │  ┌─────────┐ ┌─────────┐ ┌────────┐ │               │
                        │         │  │  Email  │ │  Slack  │ │ In-App │ │               │
                        │         │  └────┬────┘ └────┬────┘ └────┬───┘ │               │
                        │         └───────┼───────────┼───────────┼─────┘               │
                        │                 │           │           │                      │
                        │          ┌──────▼───────────▼───────────▼──────┐              │
                        │          │         Success?                    │              │
                        │          └──────────────┬──────────────────────┘              │
                        │                ┌────────┴────────┐                            │
                        │                ▼                 ▼                            │
                        │         ┌──────────┐      ┌────────────┐                      │
                        │         │ Delivered│      │ Retry/DLQ  │                      │
                        │         └──────────┘      └────────────┘                      │
                        └───────────────────────────────────────────────────────────────┘
```

## Integration Patterns

This MCP server demonstrates several patterns useful for AI agent architectures:

1. **Async Task Processing**: Notifications are queued for background processing
2. **Retry with Backoff**: Transient failures are handled with exponential backoff
3. **Dead-Letter Handling**: Permanent failures are captured for debugging
4. **Multi-Channel Delivery**: Same message can be delivered via multiple channels
5. **Priority Queuing**: High-priority notifications can be expedited
