# Monitor

Caryatid includes a monitor for observing module and topic activity. It wraps the message bus and tracks reads/writes to each topic in an in-memory `DashMap`. Useful for debugging slowness/stoppages or understanding module interactions.

## Configuration

```toml
[monitor]
output = "monitor.json"              # Write snapshots to file (optional)
topic = "caryatid.monitor.snapshot"  # Publish to RabbitMQ (optional)
frequency_secs = 5.0                 # Snapshot interval in seconds
```

You can use `output`, `topic`, or both:
- `output`: Writes JSON snapshots to a file
- `topic`: Publishes snapshots directly to the first configured RabbitMQ bus (no routing rules needed)

## Output Format

Each module lists its "reads" (topics it subscribes to) and "writes" (topics it publishes to).

**Reads:**
- `read`: Total messages read from this topic
- `unread`: Messages available on this topic (based on what other modules published; will be an underestimate if messages arrive via RabbitMQ)
- `pending_for`: How long this module has been waiting to read from this topic

**Writes:**
- `written`: Total messages written to this topic
- `pending_for`: How long this module has been waiting to write to this topic (if set, indicates congestion - a subscriber exists but isn't reading)
