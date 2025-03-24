# Standard Spy module for Caryatid

The Spy module provides a way to observe any messages on the bus.  It simply subscribes to a topic
and logs the received messages.

## Configuration

The Spy module is configured with a topic to spy on:

```toml
[module.spy]
topic = mi5.top.secret
```

## Messages

The spy module can receive any message types and logs them (as `info`) using their `Debug` trait.

## Registration

The Spy module needs to be parameterised with the type of the outer message `enum`

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),
    ... all my messages ...
}
```

Then within your `main.rs` you would register the Spy module into the
[process](../../process) like this:

```rust
    Spy::<Message>::register(&mut process);
```


