# Caryatid multi-channel subscription example

This example is the simplest possible multi-subscription model. There are three
modules, [`Publisher1`](src/publisher1.rs), [`Publisher2`](src/publisher2.rs)
and a [`Subscriber`](src/subscriber.rs). The publishers publish messages on
the topics `sample1.test` and `sample2.test` and the subscriber subscribes
to `sample1.#` and `sample2.#`. The messages are JSON objects.

## How to run it

```shell
$ cd examples/multi
$ cargo run
```

## Points of interest

In the the [subscriber](src/subscriber.rs) you can see how you can subscribe to
multiple channels using the pull interface. Note that reading from the
subscriptions is blocking and async, so assigning the futures to variables at
the top of the reading loop and then awaiting them as necessary will keep
blocking time to a minimum.
