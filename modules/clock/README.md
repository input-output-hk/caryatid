# Standard Clock module for Caryatid

The Clock module provides a regular tick event which can be used to drive time-based behaviour in other modules
without having to create your own interval system.

## Configuration

The Clock module doesn't need any configuration, it just needs to be mentioned in the top-level configuration:

```toml
[module.clock]
```

## Messages

The Clock module sends a `ClockTickMessage` once a second, which is defined in the common
[messages](../../sdk/src/messages.rs) in the SDK:

```rust
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClockTickMessage {
    /// Time of tick, UTC
    pub time: DateTime<Utc>,

    /// Tick number
    pub number: u64
}
```

The `time` is a DateTime in UTC, derived from a regular interval, which means although each tick may not be precisely one second after
the previous one, it won't drift over time, and the long-term average is exactly once per second.  The `number` increments from zero at
startup each tick, and is a handy way to derive longer intervals with `%` - e.g.

```rust
  if message.number % 60 == 0 {
     // ... happens once a minute ...
  }
```

## Registration

The Clock module needs to be parameterised with the type of an outer message `enum` which contains a `ClockTickMessage` variant, and 
provides a `From` implementation to promote one.  For example, your system-wide message enum might be:

```rust
use caryatid_sdk::messages::ClockTickMessage;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),
    // ... other messages ...
    Clock(ClockTickMessage),
}

impl From<ClockTickMessage> for Message {
    fn from(msg: ClockTickMessage) -> Self {
        Message::Clock(msg)
    }
}
```

Then within your `main.rs` you would register the Clock module into the [process](../../process) like this:

```rust
    Clock::<Message>::register(&mut process);
```

See the [typed example](../../examples/typed) to see this in action.

