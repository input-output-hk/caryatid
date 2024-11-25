# Caryatid performance test example

This example attempts to measure the raw performance of the Caryatid framework for simple
pub-sub messages passed internally and via an external bus (RabbitMQ).  It uses a multi-threaded
publisher which can send simple typed messages of a configured length, and a subscriber which just
counts messages and elapsed time and outputs the statistics when it gets a "stop" message.

## How to run it

```shell
$ cd examples/performance
$ cargo run
```

## Points of interest

You can configure the number of threads (actually Tokio tasks, which it will spread across actual
process threads) and the message size in the [configuration](performance.toml#L10):

```toml
[module.publisher]
topic = "performance.test"

# Number of messages per thread
count = 1000000

# Number of threads
threads = 1

# Length of 'data' string
length = 10000
```

By default the messages are routed internally - you can change this to external by changing the
[route](performance.toml#L29):

```toml
[[message-router.route]]
pattern = "#"
bus = "external"
```
