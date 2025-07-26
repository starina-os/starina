# Getting Started

## Install Packages

macOS and Ubuntu users can install the packages with:

```
./setup.sh
```

Other platforms are not tested, but should work as long as you have the following packages installed:

- Rust (use [rustup](https://rustup.rs/))
- QEMU
- GDB (optional)

To use Linux compatibility layer, you also need:

- Zig
- skopeo
- squashfs-tools (`mksquashfs`)
- Clang, LLD, LLVM binutils
- GNU make and more so-called *build-essential* packages

> [!TIP]
>
> **Can I use Windows?**
>
> Yes, it should just work on Windows. That said, I recommend using WSL2 for good performance and ease of setup. You can use the same commands as Ubuntu above.

## Build and Run OS

`make run` builds the OS and runs it in QEMU. Just type:

```bash
make run
```

> [!NOTE]
>
> To exit QEMU, type <kbd>Ctrl+A</kbd> then <kbd>X</kbd>. Or <kbd>C</kbd> to enter QEMU monitor (debug console).

`make run` accepts the following environment variables:

| Name | Value |  Description |
|------|--------|------|
| `BUILD_ONLY` | `1` | Do not start QEMU after building the OS. |
| `QEMU` | `/path/to/qemu` | QEMU binary path. Default is `qemu-system-riscv64`. |
| `RELEASE` | `1` | Build in release mode. Default is debug mode. |

## Debugging with GDB

`make run` starts QEMU with GDB server enabled. You can attach GDB to Starina Kernel by:

```bash
make debug
```

GDB is super useful for debugging especially when you debug the kernel and in-kernel apps. For example, if the kernel hungs, you can check the backtrace with `bt` command.

## What's Next?

TODO: How to write apps such as device drivers.
