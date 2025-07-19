# Virtio-net Device Driver

The virtio-net driver provides ethernet connectivity for Starina by implementing the VirtIO network device specification. It bridges hardware network devices with the software networking stack.

## Overview

The virtio-net driver serves as the foundation of Starina's networking capabilities:

- **Hardware Interface**: Communicates with VirtIO network devices
- **Service Provider**: Exports "device/ethernet" service for networking stacks
- **Packet Processing**: Handles network packet transmission and reception
- **Interrupt Driven**: Responds to hardware interrupts for efficient packet processing

## App Specification

```rust
pub const SPEC: AppSpec = AppSpec {
    name: "virtio_net",
    env: &[EnvItem {
        name: "device_tree",
        ty: EnvType::DeviceTree {
            matches: &[DeviceMatch::Compatible("virtio,mmio")],
        },
    }],
    exports: &[ExportItem::Service {
        service: "device/ethernet",
    }],
    main,
};
```

### Device Tree Integration

The driver uses device tree matching to automatically discover VirtIO MMIO network devices:

- **Compatible String**: Matches "virtio,mmio" devices
- **Automatic Probing**: Scans device tree at startup
- **Resource Allocation**: Maps MMIO regions and interrupt lines
- **Hardware Validation**: Verifies device type is network (DeviceType 1)

## Architecture

### Event-Driven Design

The driver handles three types of events:

```rust
enum State {
    Startup(Channel),      // New client connections
    Interrupt(Interrupt),  // Hardware interrupts 
    Upstream(ChannelReceiver), // Packet transmission requests
}
```

### Packet Flow

#### Transmission (TX)
1. **Receive Request**: Upstream sends `Message::Data` with packet
2. **Queue Packet**: Add packet to VirtIO transmit queue
3. **Notify Device**: Signal hardware to process packets
4. **Completion**: Handle transmission completion interrupts

#### Reception (RX)  
1. **Hardware Interrupt**: Device signals packet arrival
2. **Process Packets**: Read packets from VirtIO receive queue
3. **Forward Upstream**: Send `Message::Data` to network stack
4. **Replenish Buffers**: Add new receive buffers to queue

### Memory Management

```rust
const DMA_BUF_SIZE: usize = 4096;
```

The driver uses DMA buffer pools for efficient packet handling:

- **DMA Coherent Memory**: Buffers accessible by hardware
- **Buffer Recycling**: Reuses buffers to minimize allocation overhead
- **Zero-Copy**: Direct packet transfer between hardware and network stack

## VirtIO Implementation

### Device Configuration

The driver reads device configuration to determine capabilities:

```rust
struct VirtioNetConfig {
    mac: [u8; 6],           // Device MAC address
    status: u16,            // Link status
    max_virtqueue_pairs: u16, // Queue pair count
    mtu: u16,               // Maximum transmission unit
    speed: u32,             // Link speed
    duplex: u8,             // Duplex mode
}
```

### Packet Headers

VirtIO network packets include a standard header:

```rust
struct VirtioNetModernHeader {
    flags: u8,              // Packet flags
    gso_type: u8,           // Generic segmentation offload type
    hdr_len: u16,           // Header length
    gso_size: u16,          // Segmentation size
    checksum_start: u16,    // Checksum calculation start
    checksum_offset: u16,   // Checksum field offset
}
```

## Interface Protocol

### Initialization

1. **Device Discovery**: Scan device tree for VirtIO MMIO devices
2. **Device Verification**: Confirm device type is network (type 1)
3. **Feature Negotiation**: Negotiate supported VirtIO features
4. **Queue Setup**: Initialize transmit and receive virtqueues
5. **MAC Address**: Read device MAC address from configuration
6. **Interrupt Setup**: Register for device interrupts

### Client Connection

Network stacks connect to the driver via the startup channel:

```rust
// Network stack connects
Ok(Message::Connect { handle }) => {
    let (sender, receiver) = handle.split();
    // Register as upstream for packet forwarding
}
```

### Packet Transmission

Clients send packets using Data messages:

```rust
// Client sends packet
Ok(Message::Data { data }) => {
    virtio_net.transmit(data);
}
```

### Packet Reception

The driver forwards received packets to connected clients:

```rust
virtio_net.handle_interrupt(|data| {
    sender.send(Message::Data { data })?;
});
```

## Error Handling

The driver handles various error conditions gracefully:

### Channel Backpressure
```rust
if err == ErrorCode::Full {
    debug_warn!("upstream channel is full, dropping packet");
}
```

### Disconnected Clients
```rust
if readiness == Readiness::CLOSED => {
    warn!("upstream channel closed, stopping transmission");
    upstream_sender = None;
}
```

### Hardware Errors
- **Invalid Descriptors**: Log warnings and continue operation
- **DMA Failures**: Retry with new buffers
- **Device Reset**: Reinitialize device state

## Performance Characteristics

### Throughput
- **Zero-Copy Path**: Direct DMA between hardware and network stack
- **Interrupt Coalescing**: Batch packet processing for efficiency
- **Queue Depth**: Configurable virtqueue sizes for optimal performance

### Latency
- **Interrupt-Driven**: Immediate packet processing on arrival
- **Minimal Overhead**: Direct hardware-to-software packet path
- **No Polling**: Event-driven design reduces CPU usage

## QEMU Integration

The driver is designed to work seamlessly with QEMU's VirtIO network emulation:

### QEMU Configuration
```bash
-netdev user,id=net0 \
-device virtio-net-device,netdev=net0
```

### Default Network Settings
- **IP Range**: 10.0.2.0/24 (QEMU user networking)
- **Gateway**: 10.0.2.2
- **DNS**: 10.0.2.3
- **Host Access**: 10.0.2.2

## Usage Example

The virtio-net driver is typically used by the tcpip service:

```rust
// tcpip service connects to ethernet driver
pub const TCPIP_SPEC: AppSpec = AppSpec {
    name: "tcpip",
    env: &[EnvItem {
        name: "driver",
        ty: EnvType::Service { service: "device/ethernet" },
    }],
    exports: &[ExportItem::Service { service: "tcpip" }],
    main,
};
```

The startup process automatically connects them:
1. virtio-net exports "device/ethernet" 
2. tcpip requests "device/ethernet" dependency
3. Startup creates channel between them
4. tcpip can send/receive ethernet frames via the driver

## Debug Information

The driver provides extensive logging for debugging:

```rust
// MAC address discovery
debug!("MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}");

// Packet transmission
trace!("transmitting {} bytes");

// Interrupt handling  
trace!("interrupt: received interrupt");

// Error conditions
debug_warn!("upstream channel is full, dropping packet");
```

## Supported Features

### VirtIO Features
- **Basic Networking**: Packet transmission and reception
- **MAC Address**: Device-specific MAC address
- **Link Status**: Network link up/down detection
- **MTU Configuration**: Maximum transmission unit support

### Future Enhancements
- **Checksum Offload**: Hardware checksum calculation
- **TSO/GSO**: TCP segmentation offload
- **Multiple Queues**: Multi-queue networking for parallelism
- **VLAN Support**: Virtual LAN tagging

## Dependencies

- **VirtIO Library**: VirtIO specification implementation
- **Driver SDK**: DMA buffer management and MMIO access
- **Device Tree**: Hardware device discovery
- **Interrupt Handling**: Hardware interrupt management

## Next Steps

- Learn about the [tcpip service](/apps/tcpip) that uses this driver
- Explore [device tree integration](/concepts/startup) for hardware discovery
- Understand [channels](/concepts/channel) for driver-to-stack communication
- Check out [interrupt handling](/concepts/poll) for event-driven programming