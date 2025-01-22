# Caryatid message spy

This is a simple tool that can connect to the external message bus and spy on any or all
messages posted on there.

## How to run it

```shell
$ cd tools/spy
$ cargo run
```

## Configuration

In `spy.toml` you can set the topic pattern to subscribe to - it defaults to `#` which
means everything!

```toml
[module.subscriber]
topic = "#"
```

Use `#` to mean "any number of dot-separated words", `*` to mean any one dot-separated word.
