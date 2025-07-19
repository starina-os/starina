# API Server

The API server is a lightweight HTTP server that provides a web-based interface for interacting with Starina. It demonstrates how to build HTTP services using the tcpip networking stack.

## Overview

The apiserver showcases several key Starina concepts:

- **HTTP Server**: Built on top of the tcpip service for networking
- **Event-Driven Architecture**: Uses Poll for handling multiple client connections
- **Static File Serving**: Serves a web-based shell interface
- **RESTful Endpoints**: Provides API endpoints for system interaction

## App Specification

```rust
pub const SPEC: AppSpec = AppSpec {
    name: "apiserver",
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { service: "tcpip" },
    }],
    exports: &[],
    main,
};
```

The apiserver depends on the tcpip service but doesn't export any services itself - it's a leaf node in the service dependency graph.

## Architecture

### Request Processing Flow

1. **Listen Setup**: Opens TCP socket on port 80 via tcpip service
2. **Connection Handling**: Accepts new client connections
3. **HTTP Parsing**: Parses HTTP requests using custom parser
4. **Route Dispatch**: Routes requests to appropriate handlers
5. **Response Generation**: Sends HTTP responses back to clients

### Event-Driven Design

```rust
enum State {
    Tcpip(ChannelReceiver),      // Initial tcpip connection
    Listen(Channel),             // Listening socket
    Data {                       // Active client connections
        client: Mutex<Client>,
        ch: ChannelReceiver,
    },
}
```

The server uses [Poll](/concepts/poll) to efficiently handle multiple concurrent HTTP connections without blocking.

## Endpoints

The apiserver provides several endpoints:

### `/` - Web Shell
- **Method**: GET
- **Content-Type**: text/html
- **Description**: Serves the main web interface for Starina
- **Response**: Interactive HTML shell for system interaction

### `/logs` - System Logs  
- **Method**: GET
- **Content-Type**: text/plain
- **Description**: Returns recent kernel and application logs
- **Response**: Plain text log entries

### `/big` - Performance Test
- **Method**: GET  
- **Content-Type**: text/plain
- **Description**: Returns large response for testing network performance
- **Response**: Large text payload for benchmarking

## HTTP Implementation

### Custom HTTP Parser

The apiserver includes a custom HTTP/1.1 parser built for Starina's no_std environment:

```rust
struct RequestParser {
    state: ParserState,
    headers: Vec<(String, String)>,
    content_length: Option<usize>,
}
```

### Response Buffering

Responses are buffered to ensure efficient network utilization:

```rust
struct BufferedResponseWriter {
    buffer: Vec<u8>,
    headers_sent: bool,
    headers: HashMap<HeaderName, String>,
}
```

## Usage Example

Accessing the apiserver from a client:

```bash
# Get the web interface
curl http://10.0.2.15/

# View system logs
curl http://10.0.2.15/logs

# Performance test
curl http://10.0.2.15/big
```

From within Starina apps:

```rust
// Connect to apiserver via tcpip
env.tcpip.send(Message::Open {
    call_id: CallId::from(1),
    uri: b"tcp-connect:10.0.2.15:80",
}).unwrap();

// Send HTTP request
let request = b"GET /logs HTTP/1.1\r\nHost: localhost\r\n\r\n";
socket_ch.send(Message::Data { data: request }).unwrap();
```

## Web Shell Interface

The served HTML page provides an interactive web-based interface:

- **System Status**: Real-time view of Starina system state
- **Log Viewer**: Browse kernel and application logs
- **Service Monitor**: Monitor running services and their status
- **Interactive Console**: Execute commands and view results

## Configuration

The apiserver currently uses hardcoded configuration:

- **Listen Address**: `0.0.0.0:80` (all interfaces, port 80)
- **Buffer Size**: 4KB for HTTP request parsing
- **Connection Limit**: No explicit limit (handled by tcpip service)

## Performance Characteristics

- **Memory Usage**: Minimal - uses static buffers and streaming responses
- **Concurrency**: Handles multiple simultaneous HTTP connections
- **Latency**: Low latency due to event-driven architecture
- **Throughput**: Limited by underlying network stack performance

## Security Considerations

The current implementation is designed for development/demo purposes:

- **No Authentication**: All endpoints are publicly accessible
- **No HTTPS**: Uses plain HTTP (no TLS encryption)
- **Basic Input Validation**: Minimal request validation
- **Resource Limits**: No explicit DoS protection

Production deployments should add appropriate security measures.

## Integration Examples

### Building a REST API

```rust
pub fn route(req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    match (&req.method, req.path.as_str()) {
        (Method::Get, "/api/status") => handle_status(req, resp),
        (Method::Post, "/api/restart") => handle_restart(req, resp),
        (Method::Get, "/api/metrics") => handle_metrics(req, resp),
        _ => error(resp, StatusCode::new(404).unwrap(), "Not found"),
    }
}
```

### Adding Middleware

```rust
fn handle_request(req: &Request, resp: &mut impl ResponseWriter) {
    // CORS headers
    resp.headers_mut().insert(
        HeaderName::ACCESS_CONTROL_ALLOW_ORIGIN, 
        "*"
    ).unwrap();
    
    // Route to handlers
    route(req, resp).unwrap_or_else(|e| {
        error(resp, StatusCode::new(500).unwrap(), &e.to_string());
    });
}
```

## Dependencies

- **tcpip**: Networking service for TCP socket operations
- **starina**: Core Starina runtime and message passing
- **anyhow**: Error handling (compiled for no_std)

## Next Steps

- Learn about the [tcpip service](/apps/tcpip) that provides networking
- Explore [channels](/concepts/channel) for message-based communication  
- Understand [poll](/concepts/poll) for event-driven programming
- Check out the [startup process](/concepts/startup) for service discovery