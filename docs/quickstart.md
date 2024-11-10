# Quickstart

In this guide, we'll prepare a development environment and write a Hello World application.

## Clone the repository

The first step is to download the repository to your local machine. Just clone it from GitHub:

```
git clone https://github.com/starina-os/starina
```

## Install prerequisites

We support macOS, Linux (Ubuntu), and WSL2 (Ubuntu) for development. You will need to install the following tools:

- [rustup](https://rustup.rs/)
- GNU Make (`make`)
- Python (`python3`)
- QEMU (`qemu`)

### Install packages

On macOS, you can install these tools using Homebrew:

```
brew install qemu python3
```

On Ubuntu, you can install these tools using `apt`:

```
sudo apt install qemu-system python3 make
```

### Rust nightly toolchain

You also need to install the Rust nightly toolchain:

```
rustup toolchain install nightly
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
rustup component add rust-src llvm-tools
```

## Install CLI developer command (`sx`)

Starina provides a CLI developer command to streamline the development process called `sx`, like `rails` for Ruby on Rails, or `cargo` for Rust.

Enter the Starina repository and install the CLI developer command:

```
cd starina
cargo install --path cli
```

Now you can use the `sx` command to interact with the Starina project:

```
sx --help
```

## Run on QEMU

To run Starina on QEMU, just type `./sx run`:

```
$ ./sx run
[kernel      ] INFO   Starina - Faster Than "L"
[kernel      ] DEBUG  free memory: 0x0000000082942000 - 0x0000000086942000 (64 MiB)
[kernel      ] TRACE  PLIC: paddr=0xc000000

...
```

That's it!
