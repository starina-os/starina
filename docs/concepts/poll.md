# Poll

Poll is Starina's event-driven programming mechanism, similar to Linux's `epoll` or BSD's `kqueue`. It allows applications to efficiently monitor multiple handles (like channels) for readiness events.

## Overview

Instead of blocking on individual channels or using multiple threads, applications use Poll to monitor many handles simultaneously and react to events as they become ready.

```rust
use starina::poll::{Poll, Readiness};

let poll = Poll::new().unwrap();
```

## Key Concepts

### Readiness States

Poll monitors handles for different readiness states:

- **`Readiness::READABLE`**: The handle has data available to read
- **`Readiness::WRITABLE`**: The handle can accept writes without blocking  
- **`Readiness::CLOSED`**: The handle has been closed by the peer

```rust
// Monitor a channel for incoming messages
poll.add(
    channel.handle_id(),
    State::MyChannel(channel),
    Readiness::READABLE | Readiness::CLOSED,
).unwrap();
```

### State Management

Each monitored handle is associated with application-defined state. This allows you to track what each handle represents and handle events appropriately.

```rust
enum State {
    ListenChannel(Channel),
    ClientConnection(ChannelReceiver),
    NetworkSocket(SocketHandle),
}
```

## Basic Usage Pattern

1. Create a Poll instance
2. Add handles with their associated state and readiness flags
3. Enter the event loop with `poll.wait()`
4. Handle events based on the returned state and readiness

```rust
enum State {
    Server(ChannelReceiver),
    Client(ChannelReceiver),
}

fn main(env_json: &[u8]) {
    let poll = Poll::new().unwrap();
    let mut msgbuffer = MessageBuffer::new();
    
    // Add the server channel
    poll.add(
        server_ch.handle_id(),
        State::Server(server_ch),
        Readiness::READABLE | Readiness::CLOSED,
    ).unwrap();

    loop {
        let (state, readiness) = poll.wait().unwrap();
        
        match &*state {
            State::Server(ch) if readiness.contains(Readiness::READABLE) => {
                // Handle incoming server messages
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Connect { handle }) => {
                        // New client connected
                        let (_, client_rx) = handle.split();
                        poll.add(
                            client_rx.handle_id(),
                            State::Client(client_rx),
                            Readiness::READABLE | Readiness::CLOSED,
                        ).unwrap();
                    }
                    _ => {}
                }
            }
            
            State::Client(ch) if readiness.contains(Readiness::READABLE) => {
                // Handle client messages
                match ch.recv(&mut msgbuffer) {
                    Ok(message) => {
                        // Process client message
                    }
                    Err(_) => {
                        // Client disconnected
                        poll.remove(ch.handle_id()).unwrap();
                    }
                }
            }
            
            _ if readiness.contains(Readiness::CLOSED) => {
                // Handle disconnections
                poll.remove(/* handle_id */).unwrap();
            }
            
            _ => {}
        }
    }
}
```

## Benefits

### Scalability
Poll allows a single thread to handle thousands of concurrent connections efficiently, avoiding the overhead of creating threads for each connection.

### Resource Efficiency  
No need to spawn threads or use blocking I/O operations. The kernel efficiently wakes your application only when handles are ready.

### Simplicity
Unlike callback-based approaches, Poll maintains sequential program flow while providing event-driven capabilities.

## Real-World Examples

### TCP Server
The [tcpip server](/apps/tcpip) uses Poll to handle multiple socket connections and network events.

### HTTP Server  
The [apiserver](/apps/apiserver) uses Poll to manage HTTP client connections and requests.

### Device Drivers
Device drivers use Poll to respond to hardware interrupts and I/O readiness.

## Best Practices

- **Check readiness flags**: Always check which readiness flags are set before handling events
- **Handle CLOSED events**: Clean up resources when handles are closed
- **Use meaningful state**: Design your state enum to clearly represent what each handle does
- **Non-blocking operations**: Use non-blocking channel operations (`recv` that returns `WouldBlock`) 
- **Remove closed handles**: Remove handles from Poll when they're no longer needed

## Comparison with Other Systems

| System | Starina Poll | Linux epoll | BSD kqueue |
|--------|-------------|-------------|------------|
| API Style | Type-safe state | File descriptors | Event structures |
| Handles | Channels, sockets | File descriptors | Files, sockets, signals |
| State | Application-defined | External tracking | Built-in filter data |

## Next Steps

- Learn about [channels](/concepts/channel) for message passing
- Explore the [startup](/concepts/startup) process for service discovery
- Check out server implementations in [tcpip](/apps/tcpip) and [apiserver](/apps/apiserver)