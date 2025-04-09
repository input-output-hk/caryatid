# Standard Playback module for Caryatid

The Playback module provides a way to playback any messages on the bus. It
reads messages from a directory of JSON files and publishes them to a given
topic.

## Configuration

The Playback module is configured with a topic to publish to, and a directory
from which to read message files:

```toml
[module.playback]
topic = "interesting.message.channel"
path = "/path/to/playback/from/"
```

## Messages

The playback module can read any message type using their `serde::Deserialize`
trait.

## Registration

The Playback module needs to be parameterised with the type of the outer
message `enum`

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),
    ... all my messages ...
}
```

Then within your `main.rs` you would register the Playback module into the
[process](../../process) like this:

```rust
    Playback::<Message>::register(&mut process);
```


