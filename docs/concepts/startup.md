# Startup

Startup is Starina's declarative service discovery and dependency injection system. It automatically connects applications based on their service requirements and exports.

## Overview

Instead of manually creating channels and connecting services, Starina's startup process reads each app's `AppSpec` and automatically:

1. Creates channels between service providers and consumers
2. Injects dependencies as environment variables  
3. Starts applications in the correct order
4. Handles device tree matching for hardware drivers

## App Specification

Each application declares its requirements in an `AppSpec`:

```rust
use starina::spec::{AppSpec, EnvItem, EnvType, ExportItem};

pub const SPEC: AppSpec = AppSpec {
    name: "myapp",
    env: &[
        EnvItem {
            name: "storage",
            ty: EnvType::Service { service: "filesystem" },
        },
        EnvItem {
            name: "net",
            ty: EnvType::Service { service: "tcpip" },
        },
    ],
    exports: &[
        ExportItem::Service { service: "webserver" },
    ],
    main,
};
```

### Environment Items

Apps declare what they need in the `env` array:

- **`EnvType::Service`**: Requires a channel to another service
- **`EnvType::Device`**: Requires access to hardware (device tree matching)

### Exports

Apps declare what services they provide in the `exports` array:

- **`ExportItem::Service`**: Provides a service that other apps can use

## Service Resolution

The startup process connects services automatically:

```rust
// Service provider (tcpip server)
pub const TCPIP_SPEC: AppSpec = AppSpec {
    name: "tcpip",
    env: &[],
    exports: &[ExportItem::Service { service: "tcpip" }],
    main,
};

// Service consumer (apiserver)  
pub const API_SPEC: AppSpec = AppSpec {
    name: "apiserver",
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { service: "tcpip" },
    }],
    exports: &[],
    main,
};
```

At startup, the kernel:
1. Creates a channel pair for the "tcpip" service
2. Gives the server end to the tcpip app
3. Gives the client end to the apiserver app as "tcpip" environment variable

## Environment Injection

Apps receive their dependencies as JSON in the `main` function:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Env {
    pub tcpip: Channel,      // From EnvType::Service
    pub startup_ch: Channel, // Automatically provided for servers
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json)
        .expect("Failed to parse environment");
    
    // Use the injected tcpip channel
    env.tcpip.send(Message::Open { ... }).unwrap();
}
```

## Startup Channel

Service providers automatically receive a `startup_ch` channel that delivers new client connections:

```rust
// In a server app
fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).unwrap();
    
    let poll = Poll::new().unwrap();
    let (_, startup_rx) = env.startup_ch.split();
    
    poll.add(
        startup_rx.handle_id(),
        State::Startup,
        Readiness::READABLE,
    ).unwrap();
    
    loop {
        let (state, readiness) = poll.wait().unwrap();
        
        match state {
            State::Startup if readiness.contains(Readiness::READABLE) => {
                match startup_rx.recv(&mut msgbuffer) {
                    Ok(Message::Connect { handle }) => {
                        // New client connected via 'handle'
                        handle_new_client(handle);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
```

## Device Tree Integration

Hardware drivers use device tree matching:

```rust
pub const VIRTIO_NET_SPEC: AppSpec = AppSpec {
    name: "virtio_net",
    env: &[EnvItem {
        name: "device",
        ty: EnvType::Device {
            device: DeviceMatch::Compatible("virtio,mmio"),
        },
    }],
    exports: &[ExportItem::Service { service: "device/ethernet" }],
    main,
};
```

The startup process scans the device tree and provides matching hardware devices to the driver.

## Registration

Apps are registered in the kernel's startup list:

```rust
// In kernel/src/startup.rs
const INKERNEL_APPS: &[AppSpec] = &[
    hello::SPEC,
    tcpip::SPEC,
    apiserver::SPEC,
    virtio_net::SPEC,
];
```

## Benefits

### Declarative Configuration
No manual channel creation or service discovery code needed. Just declare dependencies and exports.

### Type Safety
Service names are checked at compile time, preventing typos and missing dependencies.

### Automatic Ordering
Apps start in dependency order automatically. Providers start before consumers.

### Device Management
Hardware drivers are automatically matched with appropriate devices.

## Comparison with Other Systems

| System | Starina Startup | systemd | Docker Compose |
|--------|----------------|---------|----------------|
| Style | Declarative specs | Unit files | YAML configs |
| Dependencies | Compile-time | Runtime | Runtime |
| Communication | Channels | Sockets/pipes | Network |
| Device handling | Device tree | udev rules | Host mounting |

## Example: Building a Web Stack

```rust
// Database service
const DB_SPEC: AppSpec = AppSpec {
    name: "database",
    env: &[],
    exports: &[ExportItem::Service { service: "database" }],
    main: db_main,
};

// API server that uses database
const API_SPEC: AppSpec = AppSpec {
    name: "api",
    env: &[
        EnvItem { name: "db", ty: EnvType::Service { service: "database" } },
        EnvItem { name: "net", ty: EnvType::Service { service: "tcpip" } },
    ],
    exports: &[ExportItem::Service { service: "api" }],
    main: api_main,
};

// Web frontend that uses API
const WEB_SPEC: AppSpec = AppSpec {
    name: "web",
    env: &[
        EnvItem { name: "api", ty: EnvType::Service { service: "api" } },
        EnvItem { name: "net", ty: EnvType::Service { service: "tcpip" } },
    ],
    exports: &[],
    main: web_main,
};
```

The startup process automatically:
1. Starts database first (no dependencies)
2. Starts API server with database connection
3. Starts web frontend with API connection
4. All services get tcpip connections for networking

## Next Steps

- Learn about [channels](/concepts/channel) for inter-service communication
- Explore [poll](/concepts/poll) for event-driven programming
- Check out real examples in [tcpip](/apps/tcpip) and [apiserver](/apps/apiserver)