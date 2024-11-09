# Writing Your First Application

Let's write a simple Hello World application. In this section, we'll create a simple app called `demo`.

## Scaffolding

The first step is to generate a template for your application. Starina provides `sx scaffold` to do that. Just run it with the `--type app <NAME>` option:

```
$ ./sx scaffold --type app demo
  GEN apps/demo/Cargo.toml
  GEN apps/demo/build.rs
  GEN apps/demo/main.rs
  GEN apps/demo/app.spec.json

sx: generated app at apps/demo
sx: created buildconfig.mk - edit this file to change the build configuration
sx: added apps/demo to APPS in buildconfig.mk
```

Now you have a new directory at `apps/demo` with the following files:

- `Cargo.toml`: The Cargo manifest file (see [The Cargo Book](https://doc.rust-lang.org/cargo/reference/manifest.html)).
- `build.rs`: The build script file (see [The Cargo Book](https://doc.rust-lang.org/cargo/reference/build-scripts.html)).
- `main.rs`: The main source file.
- `app.spec.json`: The Starina application manifest file.

> [!NOTE]
>
> Build configuration is stored in `buildconfig.mk`. The scaffold command automatically updates it to include the new app.

## Running the application

To run the application, execute `./sx run`:

```
$ ./sx run
...
[demo        ] INFO   Hello World!
```

## Discover Starina APIs

Starina API (`starina_api`) provides a set of useful functions. For example:

```rust
use starina_api::prelude::*; // info! and Vec

// Print a message to the debug console.
let answer = 42;
info!("answer is {}", answer);

// A variable-length array (so-called vector or list).
let mut vec = Vec::new();
vec.push(1);
vec.push(2);
vec.push(3);
info!("vec: {:?}", vec);

// HashMap (so-called dictionary or associative array).
use starina_api::collections::HashMap;
let mut map = HashMap::new();
map.insert("apple", 1000);
map.insert("banana", 2000);
map.insert("cherry", 3000);
info!("map: {:?}", map);
```

Discover more `no_std` APIs in [crates.io](https://crates.io/categories/no-std?sort=downloads) to focus on what you actually want to implement!

## Connect with services

In Linux and other major operating systems, applications calls system calls to use OS services such as file systems, TCP/IP networking, device drivers, etc. In microkernel architecture, we still use system calls, but the actual OS services are provided by separate userspace programs connected over inter-process communication (IPC).

In Starina, each service (or *server*) provides a set of APIs over a message-passing mechanism called *channel*. Channel is a bi-directional, asynchronous message queue between two processes.

Here, let's connect the `demo` application with `apps/echo` app, which is a simple server which replies a simple message, just like `ping` command in Linux.

However, how can we know the server's channel? In Starina, the service dependencies are managed through systemd/Kubernetes-like declarative configuration files called *spec files*. Declare a new dependency in `app.spec.json`:

```json
{
  "name": "demo",
  "kind": "app/v0",
  "spec": {
    "depends": [
      {
        "name": "echo",
        "type": "service",
        "interface": "echo"
      }
    ],
    "provides": []
  }
}
```

Now, Starina will automatically connect the `echo` service. You can get the channel via `Environ`, the first parameter of the `main` function:

```rust
#![no_std]
#![no_main]

use starina_api::environ::Environ;
use starina_api::prelude::*;

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("env: {:#?}", env);
    let ping_server_ch = env.take_channel("dep:echo").unwrap();
    info!("ping_server_ch: {:?}", ping_server_ch);
}
```

Run the application with `echo`:

```
$ make run APPS="apps/demo apps/echo"
...
[demo        ] INFO   env: {
    "dep:echo": Channel(
        Channel(#1),
    ),
    "dep:startup": Channel(
        Channel(#2),
    ),
}
[demo        ] INFO   ping_server_ch: Channel(#1)
```

You can see the `echo` channel is connected to the `demo` application!

> [!TIP] **What is `dep:startup`?**
>
> You may notice that there is another channel named `dep:startup`. This is a channel which is connected to the service which started the application.
>
> You will see more about this in [Writing Your First Server](writing-your-first-server) guide.

## Interface Defniition Language (IDL)

We are almost there! Now, we have a channel to the `echo` service. However, how can we know what kind of messages we can send to the server? In Starina, we use Interface Definition Language (IDL) to define the message format.

You can find the IDL file at `spec/interfaces/echo.interface.yml`. Here is the definition of `ping` call:

```json
      {
        "name": "ping",
        "context": "control",
        "type": "call",
        "params": [
          {
            "name": "value",
            "type": "int32",
            "help": "The value to return"
          }
        ],
        "returns": [
          {
            "name": "value",
            "type": "int32",
            "help": "The value returned"
          }
        ]
      },
```

`type: call` indicates that it is a Remote Procedure Call (RPC). A client sends a request, and server replies a response, ;like HTTP. `context` field is used to categorize the message, which is not utilized in `echo` interface.

Both request/resuponse messages have a single 32-bit integer field `value`. This is what we'll try!

Now we know the service protocol, you might wonder how to define the message structure in Rust. No worries! Starina will auto-generate the message structure for you in `build.rs` using `    starina_autogen::generate_for_app`, which `scaffold.py` has already done.

To import the generated code, add the following line to `main.rs`:

```rust
starina_api::autogen!();

// Import the generated code.
use starina_autogen::idl::ping::Ping;
use starina_autogen::idl::ping::PingReply;
```

This internally calls [`include!`](https://doc.rust-lang.org/std/macro.include.html) macro to include the generated code. The auto generated code will be embedded into the file directly, as `starina_autogen` module.

> [!TIP] **Why not defniing interfaces in Rust?**
>
> Rust `struct`s with procedural macros are powerful, but we prefer IDL because:
>
> - IDL is language-agnostic. We plan to support other programming languages in the future.
> - It's easier to debug and maintain the auto-generated code.
> - JSON is easier to read and write by programs. For example, we don't need to port Rust compiler to build a web-based IDL visualizer.

## Send a message to the server

You're now ready to send a message to the `echo` service! Let's send and receive a message:

```rust
#![no_std]
#![no_main]

// Embed the auto-generated code from IDL.
starina_api::autogen!();

use starina_api::environ::Environ;
use starina_api::prelude::*;
use starina_api::types::message::MessageBuffer;
// Use the auto-generated message definitions.
use starina_autogen::idl::echo::Ping;
use starina_autogen::idl::echo::PingReply;

#[no_mangle]
pub fn main(mut env: Environ) {
    // Get the channel to the echo.
    let ping_server_ch = env.take_channel("dep:echo").unwrap();
    info!("ping_server_ch: {:?}", ping_server_ch);

    // Prepare a memory buffer to receive a message.
    let mut msgbuffer = MessageBuffer::new();
    for i in 0.. {
        // Send a message to the server asynchronously.
        ping_server_ch.send(Ping { value: i }).unwrap();

        // Wait for a reply from the server.
        let reply = ping_server_ch.recv::<PingReply>(&mut msgbuffer).unwrap();

        // We've got a reply successfully!
        info!("got a reply: {:?}", reply);
    }
}
```

Run the application with `echo`. You will see infinite log messages like this:

```
$ ./sx run
...
[demo        ] INFO   ping_server_ch: Channel(#1)
[echo        ] INFO   ready
[demo        ] INFO   got a reply: PingReply { value: 0 }
[demo        ] INFO   got a reply: PingReply { value: 1 }
[demo        ] INFO   got a reply: PingReply { value: 2 }
[demo        ] INFO   got a reply: PingReply { value: 3 }
[demo        ] INFO   got a reply: PingReply { value: 4 }
...
```

It works! You've successfully written your first Starina app!

## Next steps

Interestingly, this guide covers most of what you need to know to write an Starina application. You will need to learn few more APIs to write OS services, but the basic concepts are the same: scaffold your app with `tools/scaffold.py`, fill the spec file to inject dependencies into `Environ`, and communicate with other components over channels.

[Writing Your First Serverrver](writing-your-first-server) is a good next step to learn how to write an OS service.
