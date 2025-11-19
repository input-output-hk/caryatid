# Module monitor

Caryatid supports a "monitor" to see the activity of the different modules and topics in a process. It's implemented as an overlay to the message bus, which tracks reads/writes to each topic in an in-memory `DashMap`.

This monitor is useful for debugging slowness/stoppages, or for understanding the interactions between modules. It's not intended to serve as a form of instrumentation.

Below is how to enable it:
```toml
[monitor]
output = "monitor.json" # Write to a file named monitor.json
frequency_secs = 5.0    # every 5 seconds
```

Every module shows a list of "reads" (streams it reads from) and "writes" (streams it writes to). 

We track this information for reads:
 - `read`: how many messages this module has read from this topic.
 - `unread`: how many messages are available for this module on this topic. This is based on how many messages were published by another module, it will be an underestimate if messages are sent over rabbitmq.
 - `pending_for`: how long has this module been waiting to read a message on this topic.

We track this information for writes:
- `written`: how many messages this module has written to this topic.
- `pending_for`: how long has this module been waiting to write a message to this topic. If this is set, the topic is congested; some module is subscribed to it but not reading from it.