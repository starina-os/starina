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

```rust [Cargo.toml]
[package]
name = "my_hello"

[dependencies]
starina = { workspace = true }
```

```rust [src/lib.rs]
#![no_std]

use starina::prelude::*;
use starina::spec::AppSpec;

pub const SPEC: AppSpec = AppSpec {
    name: "my_hello",
    env: &[],
    exports: &[],
    main,
};

fn main(_env: Environ) {
    info!("Hello, World!");
}
```

That's it! It's very similar to a usual Hello World in Rust, but there are some differences:

- It uses `lib.rs`, not `main.rs`. This is because Starina may run your apps like Unikernel.
- `#![no_std]` is required to use Starina. We don't support `std` yet.
- `use starina::prelude::*` imports frequently used types, including what you expect in `std` such as `Vec`, `String`, and `Box`. Moreover, it imports logging macros such as `info!`.
- `pub const SPEC` is required and defines what it depends on, what it exports to other apps, and the entrypoint (`main` function).
- `fn main(_env_json: &[u8])` is the entrypoint, but takes an environment JSON as an argument.

> ![TIP]
>
> This means