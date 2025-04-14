# Getting Started

## Install Packages

### macOS

```bash
brew install qemu riscv64-elf-gdb
```

### Ubuntu

```bash
sudo apt install qemu gdb-multiarch
```


## Setup Rust

Install [rustup](https://rustup.rs/) and the required toolchain as follows:

```bash
rustup override set nightly
rustup target add riscv64gc-unknown-none-elf
rustup component add rust-src llvm-tools
```

## Build and Run OS

`run.sh` builds the OS and runs it in QEMU. Just type:

```bash
./run.sh
```

> [!NOTE]
>
> To exit QEMU, type <kbd>Ctrl+A</kbd> then <kbd>X</kbd>. Or <kbd>C</kbd> to enter QEMU monitor (debug console).

`run.sh` accepts the following environment variables:

| Name | Value |  Description |
|------|--------|------|
| `BUILD_ONLY` | `1` | Do not start QEMU after building the OS. |
| `QEMU` | `/path/to/qemu` | QEMU binary path. Default is `qemu-system-riscv64`. |
| `RELEASE` | `1` | Build in release mode. Default is debug mode. |

## Debugging with GDB

`run.sh` starts QEMU with GDB server enabled. You can attach GDB to Starina Kernel on QEMU and start debugging as follows:

```bash
riscv64-elf-gdb
```

GDB is super useful for debugging especially when you debug the kernel and in-kernel apps. For example, if the kernel hungs, you can check the backtrace with `bt` command.

> [!IMPORTANT]
>
> Run `riscv64-elf-gdb` in the same directory as `run.sh`. There is a hidden file (`.gdbinit`) that initializes GDB for Starina automatically.

## What's Next?

TODO: How to write apps such as device drivers.
