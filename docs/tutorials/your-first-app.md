# Your First App

In this tutorial, we will walk through the steps to create your first Starina app.

## What's an app?

Applications (*apps*) are independent programs that run on the microkernel. Unlike monolithic operating systems, most of work is done by apps: OS services (called *servers*), device drivers, HTTP server, Linux containers, and user applications, and more.

Apps communicate with each other through channels, creating the software world you need. The microkernel serves simply as a runtime environment for these apps.

## Directory structure

Apps are located in the `apps` directory. Especially, miscellaneous programs are in `/apps/bin`. In this tutorial, let's create `my_hello` app:

```
mkdir -p apps/bin/my_hello
```

## Scaffold

Let's fill the directory with minimal files:

```rust [apps/bin/my_hello/Cargo.toml]
[package]
name = "my_hello"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
edition = { workspace = true }

[dependencies]
starina = { workspace = true }
serde = { workspace = true, features = ["derive"] }
```

```rust [apps/bin/my_hello/src/lib.rs]
#![no_std]

use starina::environ::Environ;
use starina::prelude::*;
use starina::spec::AppSpec;

pub const SPEC: AppSpec = AppSpec {
    name: "my_hello",
    env: &[],
    exports: &[],
    main,
};

fn main(_environ: Environ) {
    info!("Hello, World!");
}
```

That's it! It's very similar to a usual Hello World in Rust, but there are some differences:

- It uses `lib.rs`, not `main.rs`. This is because Starina may run your apps like Unikernel.
- `#![no_std]` is required to use Starina. We don't support `std` yet.
- `use starina::prelude::*` imports frequently used types, including what you expect in `std` such as `Vec`, `String`, and `Box`. Moreover, it imports logging macros such as `info!`.
- `pub const SPEC` is required and defines what it depends on, what it exports to other apps, and the entrypoint (`main` function).
- `fn main(_environ: Environ)` is the entrypoint with an *environment*. It's like environment variables, but it may more than variables, e.g. channels.


## Adding the app to the system

Currently, Starina only supports in-kernel apps that are embedded into the kernel.

To add `my_hello` to the system. Add the new crate to the workspace `Cargo.toml`:

```toml [Cargo.toml] {4,8}
[workspace]
members = [
    # ...
    "apps/bin/my_hello",
]

[workspace.dependencies]
my_hello = { path = "../apps/bin/my_hello" }
```

Add the app to the kernel `Cargo.toml`:

```toml [kernel/Cargo.toml] {2}
[dependencies]
my_hello = { workspace = true }
```

Lastly, register the app to in-kernel apps list:

```rust [kernel/src/startup.rs] {3}
const INKERNEL_APPS: &[AppSpec] = &[
    /* other apps here */,
    my_hello::SPEC,
];
```

## How to run

Building and testing the OS is easy. Just run `make run`:

```
make run
```

You should see the following output:

```
[hello       ] INFO   Hello, World!
```

## Connecting to a server

A single app alone is not very useful. In Starina world, interesting features are implemented provided by servers, a kind of apps that provide services (TCP sockets, filesystems, ethernet drivers, etc.) to others.

In this tutorial, let's try a simple service `echo`. As the name suggests, it simply echoes back whatever you send to it. To access the service, tell Starina to provide us a channel to the server:

```rust [apps/bin/my_hello/src/lib.rs] {1-2,7-12}
use starina::spec::EnvItem;
use starina::spec::EnvType;

pub const SPEC: AppSpec = AppSpec {
    name: "my_hello",
    env: &[
        EnvItem {
            name: "echo_server",
            ty: EnvType::Service {
                service: "echo",
            },
        },
    ],
    exports: &[],
    main,
};
```

`env` is an array of environment items. In this case, it defines an item named `echo_server`, a channel connected to the `echo` server.

```rust [apps/bin/my_hello/src/lib.rs] {1,3-6,9-11}
use starina::channel::Channel;

#[derive(serde::Deserialize)]
struct Env {
    pub echo_server: Channel,
}

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("failed to parse environment");
    let echo_server = env.echo_server;
    info!("got echo server channel: {:?}", echo_server);
}
```

You'll see a log message like:

```
$ make run
...
[hello       ] INFO   got echo server channel: Channel(OwnedHandle(HandleId(1)))
```

## Sending a message

Now, let's send a message to the echo server using the channel:

```rust [apps/bin/my_hello/src/lib.rs] {1,7}
use starina::message::Message;

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("failed to parse environment");
    let echo_server = env.echo_server;

    echo_server.send(Message::Data { data: b"Hello from my_hello" }).unwrap();
}
```

Invoke `Channel::send` method, that's it. Key takeaways:

- Channel is a bi-directional message queue. The message will be sent to the received queue of the peer channel.
- `Message` is a type that represents a message. We have few message types, and here we use `Data` to send a byte array.
- Message delivery is asynchronous. It returns immediately after the message is enqueued.
- If the peer channel's queue is full, `send` will return an error immediately.

> [!TIP]
>
> See [Channel](/concepts/channel) for more details.

## Waiting for a message

We've sent a message to the echo server, and of course we want a reply. However, before receiving a message, we need to wait for the echo server to send a message because the receive operation is also non-blocking: if it's empty, it will return an error immediately without waiting.

The solution is to use the (effectively) only one blocking operation in Starina: `Poll::wait`. It's similar to `epoll` in Linux, an event listener in JavaScript, and `select` in async Rust and Go.

Let's register a channel to the `Poll` object, and wait for the channel to be ready for receiving a message:

```rust [apps/bin/my_hello/src/lib.rs] {1-3,5-9,18-19,21-22,24-30}
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::handle::Handleable; // for handle_id()

/// Per-channel state.
enum State {
    /// A channel connected to the echo server.
    EchoServer,
}

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("failed to parse environment");
    let echo_server = env.echo_server;

    // Send a message to the echo server.
    echo_server.send(Message::Data { data: b"Hello from my_hello" }).unwrap();

    // Create a Poll object.
    let poll = Poll::new().unwrap();

    // Register the channel to the Poll object.
    poll.add(echo_server.handle_id(), State::EchoServer, Readiness::READABLE);

    // Wait for the channel to be ready for receiving a message...
    let (state, readiness) = poll.wait().unwrap();
    match *state {
        State::EchoServer => {
            info!("echo server channel is now: {:?}", readiness);
        }
    }
}
```

And you'll see a log message like:

```
$ make run
...
[hello       ] INFO   echo server channel is now: R
```

Here, `R` is a short for `Readiness::READABLE`, which means the channel is now ready for receiving a message!

## Receiving a message

Now we know our channel has a message to receive. Let's read it:

```rust [apps/bin/my_hello/src/lib.rs] {1,25-37}
use starina::message::MessageBuffer;

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("failed to parse environment");
    let echo_server = env.echo_server;

    // Send a message to the echo server.
    echo_server.send(Message::Data { data: b"Hello from my_hello" }).unwrap();

    // Create a Poll object.
    let poll = Poll::new().unwrap();

    // Register the channel to the Poll object.
    poll.add(echo_server.handle_id(), State::EchoServer, Readiness::READABLE);

    // Wait for the channel to be ready for receiving a message...
    let (state, readiness) = poll.wait().unwrap();
    match *state {
        State::EchoServer => {
            info!("echo server channel is now: {:?}", readiness);
        }
    }

    // Receive a message from the echo server.
    let mut msgbuffer = MessageBuffer::new();
    match echo_server.recv(&mut msgbuffer) {
        Ok(Message::Data { data }) => {
            let data_str = core::str::from_utf8(data).unwrap();
            info!("received a reply from echo server: {}", data_str);
        }
        Ok(m) => {
            panic!("unexpected reply from echo server: {:?}", m);
        }
        Err(err) => {
            panic!("recv error: {:?}", err);
        }
    }
}
```

`Channel::recv` returns a message in the kernel queue. We receive one message here for simplicity, but the logic is similar in real apps too: a loop of `Poll::wait` and `Channel::recv`.

> [!TIP]
>
> `MessageBuffer` is a temporary buffer where the kernel writes a received message. What you get in `Message::Data` is a reference to the buffer's memory. This was added to simplify buffer management.

You should see this fancy log message:

```
$ make run
...
[hello       ] INFO   received a reply from echo server: Hello from my_hello
```

Yay! We've successfully used the echo service!
