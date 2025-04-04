# Standard Record module for Caryatid

The Record module provides a way to record any messages on the bus.  It simply
subscribes to a topic and logs the received messages to JSON files in a given
directory.

## Configuration

The Record module is configured with a topic to record, and a directory to
record to:

```toml
[module.record]
topic = "interesting.message.channel"
path = "/path/to/record/to/"
```

## Messages

The record module can receive any message type and records them as JSON using
their `serde::Serialize` trait.

## Registration

The Record module needs to be parameterised with the type of the outer message
`enum`

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),
    ... all my messages ...
}
```

Then within your `main.rs` you would register the Record module into the
[process](../../process) like this:

```rust
    Record::<Message>::register(&mut process);
```


