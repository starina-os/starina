# Running Linux Containers

Starina provides a lightweight Linux compatibility layer that can run Linux containers and binaries. This enables running existing Linux software on Starina without modification.

## Overview

Starina's Linux compatibility is built on:

- **Lightweight VM**: Minimal Linux kernel in a virtual machine
- **Container Runtime**: Docker-compatible container execution
- **Library Interface**: Ergonomic `std::process::Command`-like API
- **Integration**: Seamless interaction with native Starina services

## Quick Start

Set environment variables to specify the container:

```bash
export LINUXRUN_IMAGE="docker://hello-world:latest"
export LINUXRUN_ENTRYPOINT="/hello"
./run.sh
```

This runs the hello-world container automatically when Starina boots.

## Supported Container Sources

### Docker Hub Images
```bash
export LINUXRUN_IMAGE="docker://ubuntu:20.04"
export LINUXRUN_ENTRYPOINT="/bin/bash"
```

### Local Container Images
```bash
export LINUXRUN_IMAGE="file:///path/to/container.tar"
export LINUXRUN_ENTRYPOINT="/usr/bin/myapp"
```

### OCI Image Format
```bash
export LINUXRUN_IMAGE="oci:///path/to/oci-image"
export LINUXRUN_ENTRYPOINT="/app/start.sh"
```

## Architecture

### Virtual Machine Layer

Starina runs a minimal Linux kernel in a virtual machine:

- **RISC-V Linux**: Compiled for RISC-V64 architecture
- **Minimal Configuration**: Only essential drivers and subsystems
- **Container Focus**: Optimized for container execution
- **Fast Boot**: Reduced startup time for containers

### Container Runtime

The Linux VM includes a container runtime that:

- **Pulls Images**: Downloads container images from registries
- **Extracts Layers**: Unpacks container filesystem layers
- **Sets Up Environment**: Configures container environment variables
- **Executes Process**: Runs the specified entrypoint

### Integration Bridge

Communication between Starina and Linux containers:

- **Network Bridge**: Shared network stack via virtio-net
- **File System**: Shared storage through virtio-fs (planned)
- **Process Communication**: Channel-based IPC (future)

## Programming Interface

### Command-like API

Run Linux processes from Starina applications:

```rust
use linux::Command;

// Run a simple command
let output = Command::new("ls")
    .arg("-la")
    .arg("/usr/bin")
    .output()
    .expect("Failed to execute command");

println!("Output: {}", String::from_utf8_lossy(&output.stdout));
```

### Container Execution

Run entire containers programmatically:

```rust
use linux::Container;

let container = Container::new("ubuntu:20.04")
    .entrypoint("/bin/bash")
    .arg("-c")
    .arg("echo 'Hello from container'")
    .env("MY_VAR", "my_value")
    .run()
    .expect("Failed to run container");

let output = container.wait_with_output().unwrap();
println!("Container output: {}", String::from_utf8_lossy(&output.stdout));
```

## Configuration

### Environment Variables

Control container execution through environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `LINUXRUN_IMAGE` | Container image to run | `docker://nginx:latest` |
| `LINUXRUN_ENTRYPOINT` | Container entrypoint | `/usr/sbin/nginx` |
| `LINUXRUN_ARGS` | Container arguments | `-g daemon off;` |
| `LINUXRUN_ENV` | Environment variables | `KEY1=value1,KEY2=value2` |

### Resource Limits

Configure VM and container resources:

```bash
export LINUXRUN_MEMORY="512M"
export LINUXRUN_CPUS="2"  
export LINUXRUN_DISK="1G"
```

## Use Cases

### Web Applications

Run existing web frameworks:

```bash
export LINUXRUN_IMAGE="docker://node:16"
export LINUXRUN_ENTRYPOINT="/usr/local/bin/node"
export LINUXRUN_ARGS="server.js"
./run.sh
```

### Development Tools

Use familiar development environments:

```bash
export LINUXRUN_IMAGE="docker://python:3.9"
export LINUXRUN_ENTRYPOINT="/usr/local/bin/python"
export LINUXRUN_ARGS="my_script.py"
```

### Legacy Applications

Run existing Linux binaries without modification:

```bash
export LINUXRUN_IMAGE="docker://mycompany/legacy-app:latest"
export LINUXRUN_ENTRYPOINT="/app/legacy-binary"
```

## Performance Characteristics

### Memory Usage
- **Minimal Overhead**: Lightweight Linux kernel (< 50MB)
- **Shared Resources**: Efficient memory sharing with host
- **Container Density**: Run multiple containers efficiently

### Startup Time
- **Fast Boot**: Optimized Linux kernel boot (< 2 seconds)
- **Container Cache**: Cached container layers for faster startup
- **Lazy Loading**: Load container data on demand

### Network Performance
- **Native Performance**: Direct virtio-net integration
- **Zero-Copy**: Efficient packet forwarding
- **Full Compatibility**: Standard Linux networking stack

## Limitations

### Current Restrictions
- **Single Container**: One container per VM instance
- **Limited Isolation**: Shared kernel between containers
- **No Orchestration**: No built-in container orchestration
- **Architecture**: RISC-V64 containers only

### Future Enhancements
- **Multi-Container**: Multiple containers per VM
- **Container Orchestration**: Kubernetes-like orchestration
- **Cross-Architecture**: x86_64 and ARM64 support
- **Advanced Networking**: Container-to-container networking

## Integration Examples

### HTTP Proxy

Route requests from Starina to Linux containers:

```rust
// Starina HTTP server
let response = match request.path() {
    "/api/*" => {
        // Forward to Linux container
        linux_container.proxy_request(request).await
    }
    _ => {
        // Handle natively in Starina
        starina_handler.handle(request).await
    }
};
```

### Data Processing

Use Linux tools for data processing:

```rust
// Process data with Linux tools
let result = Command::new("docker")
    .arg("run")
    .arg("--rm")
    .arg("python:3.9")
    .arg("python")
    .arg("-c")
    .arg("import pandas; print(pandas.read_csv('/data/input.csv').describe())")
    .output()
    .expect("Data processing failed");
```

## Troubleshooting

### Container Won't Start

Check image format and entrypoint:

```bash
# Verify image exists
docker pull ubuntu:20.04

# Test entrypoint locally
docker run --rm ubuntu:20.04 /bin/echo "test"
```

### Network Issues

Verify network connectivity:

```bash
# Test from within container
export LINUXRUN_IMAGE="docker://alpine:latest"
export LINUXRUN_ENTRYPOINT="/bin/ping"
export LINUXRUN_ARGS="-c 3 google.com"
./run.sh
```

### Performance Problems

Monitor resource usage:

```bash
# Check memory usage
echo "Memory usage:" && cat /proc/meminfo | grep MemAvailable

# Check CPU usage  
echo "CPU usage:" && top -n 1 | head -5
```

## Security Considerations

### Container Isolation
- **Shared Kernel**: Containers share the Linux kernel
- **Limited Namespaces**: Basic process and network isolation
- **No AppArmor/SELinux**: Simplified security model

### Network Security
- **Shared Network**: Containers share network stack
- **No Firewall**: No built-in container firewalling
- **Process Trust**: Containers run with elevated privileges

### Best Practices
- **Minimal Images**: Use minimal base images
- **Read-Only Root**: Mount root filesystem read-only
- **Secrets Management**: Avoid embedding secrets in images
- **Regular Updates**: Keep base images updated

## Development Workflow

### Testing Containers

Test containers locally before deployment:

```bash
# Test locally with Docker
docker run --rm my-app:latest

# Test with Starina
export LINUXRUN_IMAGE="docker://my-app:latest"
./run.sh
```

### Building Custom Images

Create optimized images for Starina:

```dockerfile
# Dockerfile for Starina
FROM alpine:latest
RUN apk add --no-cache my-dependencies
COPY app /app/
ENTRYPOINT ["/app/myapp"]
```

### CI/CD Integration

Integrate with build pipelines:

```yaml
# CI pipeline
- name: Test on Starina
  run: |
    export LINUXRUN_IMAGE="docker://$IMAGE_NAME:$BUILD_TAG"
    export LINUXRUN_ENTRYPOINT="/app/test"
    ./run.sh
```

## Next Steps

- Learn about [Starina's architecture](/getting-started) for native development
- Explore [channel communication](/concepts/channel) for service integration
- Check out the [contributor's guide](/contributors-guide/kernel-development) for Linux kernel customization