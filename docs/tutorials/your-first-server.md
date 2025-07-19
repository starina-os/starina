# Your First Server

Learn how to create a service in Starina that other apps can communicate with using channels.

## Creating a Simple Echo Server

We'll build an echo server that accepts messages and sends them back to clients.

## App Structure

Create the directory and files:

```bash
mkdir -p apps/servers/echo/src
```

## Cargo.toml

Create `apps/servers/echo/Cargo.toml`:

```toml
[package]
name = "echo"
version = { workspace = true }
authors = { workspace = true }
edition = { workspace = true }

[dependencies]
starina = { workspace = true }
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
```

## Server Implementation

Create `apps/servers/echo/src/lib.rs`:

```rust
#![no_std]

use serde::Deserialize;
use starina::channel::{Channel, ChannelReceiver};
use starina::message::{Message, MessageBuffer, CallId};
use starina::poll::{Poll, Readiness};
use starina::spec::{AppSpec, ExportItem};
use starina::prelude::*;

pub const SPEC: AppSpec = AppSpec {
    name: "echo",
    env: &[],
    exports: &[ExportItem::Service { service: "echo" }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub startup_ch: Channel,
}

enum State {
    Startup(Channel),
    Client(ChannelReceiver),
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json)
        .expect("Failed to parse environment");

    let mut msgbuffer = MessageBuffer::new();
    let poll = Poll::new().unwrap();

    let (startup_tx, startup_rx) = env.startup_ch.split();
    poll.add(
        startup_rx.handle_id(),
        State::Startup(startup_tx),
        Readiness::READABLE | Readiness::CLOSED,
    ).unwrap();

    info!("Echo server starting...");

    loop {
        let (state, readiness) = poll.wait().unwrap();

        match &*state {
            State::Startup(startup_tx) if readiness.contains(Readiness::READABLE) => {
                // Handle new client connections
                match startup_rx.recv(&mut msgbuffer) {
                    Ok(Message::Connect { handle }) => {
                        info!("New client connected");
                        let (client_tx, client_rx) = handle.split();
                        
                        poll.add(
                            client_rx.handle_id(),
                            State::Client(client_rx),
                            Readiness::READABLE | Readiness::CLOSED,
                        ).unwrap();
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected startup message: {:?}", msg);
                    }
                    Err(_) => {
                        debug_warn!("startup channel error");
                    }
                }
            }

            State::Client(client_rx) if readiness.contains(Readiness::READABLE) => {
                // Handle client messages
                match client_rx.recv(&mut msgbuffer) {
                    Ok(Message::Call { call_id, data }) => {
                        info!("Echoing message: {:?}", core::str::from_utf8(data));
                        
                        // Echo the message back
                        let client_tx = client_rx.channel_tx();
                        client_tx.send(Message::Reply {
                            call_id,
                            data,
                        }).unwrap();
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected client message: {:?}", msg);
                    }
                    Err(_) => {
                        info!("Client disconnected");
                        poll.remove(client_rx.handle_id()).unwrap();
                    }
                }
            }

            _ if readiness.contains(Readiness::CLOSED) => {
                info!("Connection closed");
            }

            _ => {}
        }
    }
}
```

## Using Your Echo Server

To use your echo server from another app:

```rust
use starina::spec::{AppSpec, EnvItem, EnvType};

pub const SPEC: AppSpec = AppSpec {
    name: "echo_client",
    env: &[EnvItem {
        name: "echo",
        ty: EnvType::Service { service: "echo" },
    }],
    exports: &[],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub echo: Channel,
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).unwrap();
    
    // Send a message to the echo server
    let call_id = CallId::from(1);
    env.echo.send(Message::Call {
        call_id,
        data: b"Hello, Echo!",
    }).unwrap();

    // Wait for the response
    let mut msgbuffer = MessageBuffer::new();
    match env.echo.recv(&mut msgbuffer).unwrap() {
        Message::Reply { call_id, data } => {
            info!("Echo reply: {}", core::str::from_utf8(data).unwrap());
        }
        _ => {}
    }
}
```

## Key Concepts

- **Service Export**: Use `ExportItem::Service` to make your app discoverable by name
- **Event-Driven Programming**: Use `Poll` to handle multiple clients efficiently  
- **Channel Communication**: Split channels into tx/rx pairs for bidirectional communication
- **Message Types**: Use `Message::Call` and `Message::Reply` for request-response patterns

## Adding to the System

Register your server in `kernel/src/startup.rs`:

```rust
const INKERNEL_APPS: &[AppSpec] = &[
    echo::SPEC,
    // ... other apps
];
```

Build and run to see your echo server in action:

```bash
./run.sh
```

## Next Steps

- Learn about [channels](/concepts/channel) for advanced message passing
- Explore [poll](/concepts/poll) for event-driven programming patterns
- Check out existing servers like [tcpip](/apps/tcpip) and [apiserver](/apps/apiserver)