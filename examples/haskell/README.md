# Caryatid Haskell interop example

This is a very quick-and-dirty demonstration of receiving and sending typed messages
from the Caryatid framework in a Haskell program.  You'll need `cabal` installed to run it.

## To run

In one terminal run

```shell
$ cd examples/haskell
$ cabal run
```

In another, run the [typed](../typed) example:

```shell
$ cd examples/typed
$ cargo run
```

## Points of interest

When you run the Rust 'typed' example you should see this on the Haskell log:

```
Received: TestMessage (Test {dataField = "Hello, world!", numberField = 42})
Response sent.
```

then on the Rust log, you should see

```
2024-11-26T11:04:53.591967Z  INFO typed_example::subscriber: Received test: Hello from Haskell! 42
```

intermingled with the other messages received internally and externally.

### CBOR serialisation/deserialisation

You'll see that much of the [Haskell code](app/Main.hs) is taken up with manual decoding and
encoding of the message in CBOR.  The next step will be to use a CDDL tool to generate this
automatically.
