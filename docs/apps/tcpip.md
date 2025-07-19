# TCP/IP Server

The TCP/IP server provides network connectivity to Starina applications through a message-based interface. It implements a TCP/IP stack using the smoltcp library and communicates with network device drivers.

## Overview

The tcpip server acts as the central networking hub in Starina:

- **Service Provider**: Exports "tcpip" service for other apps to use
- **Device Consumer**: Uses "device/ethernet" service from network drivers  
- **Protocol Implementation**: Implements TCP/IP stack with smoltcp
- **Message Interface**: Provides URI-based networking API

## App Specification

```rust
pub const SPEC: AppSpec = AppSpec {
    name: "tcpip",
    env: &[EnvItem {
        name: "driver",
        ty: EnvType::Service {
            service: "device/ethernet",
        },
    }],
    exports: &[ExportItem::Service { service: "tcpip" }],
    main,
};
```

## Network Interface

The tcpip server provides a URI-based API for network operations:

### TCP Listen

Open a TCP listening socket:

```rust
// Listen on all interfaces, port 80
tcpip_ch.send(Message::Open {
    call_id: CallId::from(1),
    uri: b"tcp-listen:0.0.0.0:80",
}).unwrap();

// Expect OpenReply with a listening channel
match tcpip_ch.recv(&mut buffer).unwrap() {
    Message::OpenReply { call_id, handle } => {
        // 'handle' is the listening socket channel
        listen_on_socket(handle);
    }
    _ => {}
}
```

### TCP Connect

Connect to a remote TCP server:

```rust
// Connect to example.com:80
tcpip_ch.send(Message::Open {
    call_id: CallId::from(2), 
    uri: b"tcp-connect:example.com:80",
}).unwrap();

// Expect OpenReply with connection channel
match tcpip_ch.recv(&mut buffer).unwrap() {
    Message::OpenReply { call_id, handle } => {
        // 'handle' is the connected socket channel
        communicate_with_server(handle);
    }
    _ => {}
}
```

## Architecture

### Event-Driven Design

The tcpip server uses [Poll](/concepts/poll) to handle multiple concurrent operations:

```rust
enum State {
    Startup(Channel),
    Driver(ChannelReceiver),
    Control(Channel),
    Listen(ChannelReceiver),
    Data {
        smol_handle: SocketHandle,
        ch: ChannelReceiver,
    },
}
```

### Network Stack Integration

- **smoltcp**: Provides the TCP/IP protocol implementation
- **Device Interface**: Communicates with ethernet drivers via channels
- **Socket Management**: Maps Starina channels to smoltcp sockets
- **Packet Processing**: Handles network packet receive/transmit

### Message Flow

1. **App requests connection**: Sends `Message::Open` with URI
2. **Socket creation**: tcpip creates smoltcp socket  
3. **Channel return**: Returns new channel via `Message::OpenReply`
4. **Data transfer**: App sends/receives `Message::Data` on socket channel
5. **Connection cleanup**: Channel closure triggers socket cleanup

## Supported URI Schemes

| URI Scheme | Description | Example |
|------------|-------------|---------|
| `tcp-listen` | Create TCP listening socket | `tcp-listen:0.0.0.0:8080` |
| `tcp-connect` | Connect to TCP server | `tcp-connect:192.168.1.1:80` |

## Error Handling

Network errors are reported through standard message types:

```rust
// Connection failed
Message::Abort { call_id, error } => {
    println!("Connection failed with error: {}", error);
}

// Runtime error on established connection  
Message::Error { error } => {
    println!("Socket error: {}", error);
}
```

## Example Usage

Simple HTTP client using tcpip:

```rust
use starina::spec::{AppSpec, EnvItem, EnvType};

pub const SPEC: AppSpec = AppSpec {
    name: "http_client",
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { service: "tcpip" },
    }],
    exports: &[],
    main,
};

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).unwrap();
    
    // Connect to web server
    let call_id = CallId::from(1);
    env.tcpip.send(Message::Open {
        call_id,
        uri: b"tcp-connect:example.com:80",
    }).unwrap();
    
    // Get connection channel
    let socket_ch = match env.tcpip.recv(&mut buffer).unwrap() {
        Message::OpenReply { handle, .. } => handle,
        _ => panic!("Connection failed"),
    };
    
    // Send HTTP request
    let request = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    socket_ch.send(Message::Data { data: request }).unwrap();
    
    // Read HTTP response  
    match socket_ch.recv(&mut buffer).unwrap() {
        Message::Data { data } => {
            println!("Response: {}", core::str::from_utf8(data).unwrap());
        }
        _ => {}
    }
}
```

## Dependencies

- **smoltcp**: Network protocol implementation
- **virtio-net**: Default ethernet driver (via "device/ethernet" service)
- **Kernel timers**: For TCP timeouts and retransmission

## Configuration

The tcpip server currently uses hardcoded configuration:

- **IP Address**: 10.0.2.15/24 (QEMU default)
- **Gateway**: 10.0.2.2
- **MAC Address**: Auto-generated

Future versions will support dynamic configuration through device tree or configuration channels.

## Performance

- **Zero-copy**: Direct packet forwarding between driver and smoltcp
- **Event-driven**: Single-threaded async processing
- **Connection pooling**: Efficient socket handle reuse

## Next Steps

- Learn about the [virtio-net driver](/apps/virtio-net) that provides ethernet connectivity
- Explore the [apiserver](/apps/apiserver) that uses tcpip for HTTP serving
- Understand [channels](/concepts/channel) for message-based communication