# Caryatid Event-based Modular Framework

This is the Caryatid event-based, modular framework for Rust.  It allows creation of microservices
comprising one or more modules which communicate over publish-subscribe, with an optional
request/response layer on top.

Modules in the same microservice can communicate over an internal, zero-copy, in-memory bus.
Modules in different microservices can communicate over external message buses (only
RabbitMQ is implemented so far).  The module itself doesn't need to know which is in use, because
there is a routing layer to direct particular topics to different buses.

Messages are generic, and can either be a universal format such as JSON, or for better performance,
an `enum` of application-specific Rust types.  In either case, they are automatically serialised
to CBOR when passed externally.

Currently - because `tokio` doesn't play well with dynamic loading, and complexities around ABI -
microservice *processes* are built with a simple `main.rs` which explicitly loads the modules
required.  The eventual aim is to support dynamic loading at runtime into a standard process.

## Building and running

Caryatid is a standard Rust workspace, so you can build it all with

```
$ cargo build
```

at the root directory.

Individual examples can be built and run from their directories in `examples`:

```
$ cd examples/simple
$ cargo run
... builds ...
2024-11-19T13:01:43.547798Z  INFO simple_example: Caryatid modular framework - simple example process
... runs ...
```

## Code structure

The code is structured as follows:

* [`sdk`](./sdk): the SDK required to build modules - mostly traits and macros
* [`process`](./process): a library for building a microservice process.  Most of the implementation is in here.
* [`modules`](./modules): standard modules likely to be useful across projects
* [`examples`](./examples): simple example code demonstrating various features

### Examples

* [`examples/simple`](./examples/simple): Simplest possible pub-sub with JSON
* [`examples/typed`](./examples/typed): Pub-sub with typed messages
* [`examples/request`](./examples/request): Request response with JSON
* [`examples/rest`](./examples/rest): A modular REST server
* [`examples/performance`](./examples/performance): Performance testing
