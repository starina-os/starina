# Starina

Starina (named after [stellina](https://en.wiktionary.org/wiki/stellina)), is a general-purpose, microkernel-based, modern operating system designed for developers. It aims to be a production-ready OS, and more importantly, a fun and easy-to-understand OS where you can enjoy the development as if you are writing a Web application.

## Philosophy

The ultimate goal of this project is to create a production-ready OS to be a good alternative to real-world OSes. To make this happen, Starina values the following principles:

- **Userspace-first:** Most OS components live in userspace for better developer experience. The microkernel simply provides a runtime for applications.
- **Simplicity over perfection:** Straightforward design that covers common use cases. Make it work first, optimize later.
- **Incrementally adoptable:** Easy integration with existing systems to facilitate gradual adoption.

## 2025 Roadmap

This year we're focusing on cloud computing, with [starina.dev](https://starina.dev) running on Starina's Linux compatibility layer:

![Architecture](./docs/architecture.svg)

- [x] Microkernel prototype in Rust ([starina.dev](https://starina.dev) running on Linux/QEMU!)
- [x] Complete redesign and rewrite
- [x] Rust-based zero-cost isolation ([unikernel](https://en.wikipedia.org/wiki/Unikernel) style)
- [x] Device tree support
- [x] Asynchronous message passing + epoll-like event driven API
- [x] Declarative OS service discovery
- [x] TCP/IP networking
- [x] Virtio-net device dirver
- [x] WSL2-like Linux compatibility layer
- [ ] **WIP:** Linux container image support (`docker run`-like experience)
- [ ] Shell (in an unopinionated headless Web-based approach)
- [ ] File system server
- [ ] TypeScript (WebAssembly-based) or Swift (Embedded Swift) API
- [ ] Traditional usermode-based isolation

## Getting Started

```bash
# Install dependencies
brew install qemu riscv64-elf-gdb  # macOS
apt install qemu gdb-multiarch     # Ubuntu

# Build and run
./run.sh

# Debug with GDB
riscv64-elf-gdb -ex bt
```

## Is it Linux or POSIX compatible?

Starina uses a completely new API design and is not POSIX-compatible. However, you can run existing Linux applications using its lightweight VM based Linux compatibility library with an ergonomic `std::process::Command`-like API. Learn more in [this blog post](https://seiya.me/blog/hypervisor-as-a-library).

## Why Rust?

We (and perhaps you too) love to debate the best text editor and programming language, sometimes very seriously and passionately.

Starina is entirely written in Rust because it is *"C++ with seatbelts"*, which is suitable for building a robust yet high-performance OS. Seatbelts are sometimes annoying indeed, but we know it saved us from countless bugs by enforcing good practices. Notably, I don't need address sanitizer when writing Rust. That's a huge factor for me.

That said, it's crystal clear that Rust (or any other language) is not the best language for everything. That's why Starina is designed to be language-agnostic, and I plan to add seamless support for other languages such as TypeScript. What if you can prototype OS components such as device drivers, as if you are writing a Web app? Isn't that cool?
