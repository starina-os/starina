# Your First App

In Starina, *apps* are the fundamental building blocks of the entire system. Unlike traditional operating systems, most of work is done by apps: OS services (called *servers*), device drivers, HTTP server, Linux containers, and user applications.

Apps communicate with each other through channels, creating the software world you are familiar with. The microkernel serves simply as a runtime environment for these apps.

In this tutorial, we will walk through the steps to create a simple "Hello World" app.

## Creating the App Structure

Create a new directory for your app:

```bash
mkdir -p apps/bin/hello/src
```

## Writing the Cargo.toml

Create `apps/bin/hello/Cargo.toml`:

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[dependencies]
starina = { path = "../../../libs/rust/starina" }
```

## Writing Your App

Create `apps/bin/hello/src/lib.rs`:

```rust
#![no_std]

use starina::prelude::*;
use starina::spec::AppSpec;

pub const SPEC: AppSpec = AppSpec {
    name: "hello",
    env: &[],
    exports: &[],
    main,
};

fn main(_env_json: &[u8]) {
    info!("Hello, World!");
}
```

## Understanding the Code

- `#![no_std]`: Starina apps run in a minimal runtime without standard library
- `AppSpec`: Declares your app's metadata - name, dependencies (env), and exports
- `env: &[]`: No service dependencies needed for this simple app
- `exports: &[]`: This app doesn't provide any services to other apps
- `main`: Entry point function that receives environment configuration as JSON

## Building and Running

Add your app to the kernel's startup list in `kernel/src/startup.rs`:

```rust
const INKERNEL_APPS: &[AppSpec] = &[
    hello::SPEC,
    // ... other apps
];
```

Build and run:

```bash
./run.sh
```

You should see "Hello, World!" in the kernel logs.

## Next Steps

- Add service dependencies to communicate with other apps
- Create services that other apps can use
- Learn about [channels](/concepts/channel) for inter-app communication