# Starina

Starina (named after [stellina](https://en.wiktionary.org/wiki/stellina)), is a general-purpose, microkernel-based, modern operating system designed for developers. It aims to be a production-ready OS, and more importantly, a fun and easy-to-understand OS where you can enjoy the development as if you are writing a Web application.

## Goals

The ultimate goal of this project is to create a production-ready OS to be a good alternative to real-world OSes. To make this happen, Starina values the following principles:

- **Userspace-first approach:** Make OS development approachable and fun for everyone. Prioritize developer experience in the userspace, where the most OS components reside. The microkernel is just a runtime for applications.
- **Simplicity over perfection:** Emphasize a straightforward design which covers the most common use cases. Make it work first. Make it better later.
- **Incrementally adoptable:** Facilitate easy adoption of Starina by providing a seamless integration with existing systems.

## Roadmap for 2025

This year, we focus on cloud computing domain, where Starina will be used as a tiny runtime for Linux containers.

- [x] Prototyping an microkernel-based OS in Rust: [https://starina.dev](https://starina.dev) is served by Starina on Linux/QEMU hypervisor!
- [x] Redesign the OS based on lessons learned
- [x] Rewrite from scratch
- [x] Rust-based almost-zero-cost isolation ([Unikernel](https://en.wikipedia.org/wiki/Unikernel) style)
- [x] TCP/IP server
- [ ] Wrap up APIs **(work in progress)**
- [ ] WSL2-like Linux compatibility layer
- [ ] File system server
- [ ] TypeScript API + language-based isolation (akin to WebAssembly)
- [ ] Usermode isolation (traditional microkernel style)
- [ ] Shell
- [ ] Streamlined observability and debugging experience

## How to run

```bash
# Install dependencies
brew install qemu riscv64-elf-gdb # Ubuntu: apt install qemu gdb-multiarch

# Build and run (with GDB server enabled)
./run.sh

# Attach GDB to QEMU and start debugging
riscv64-elf-gdb -ex bt
```

## Is it Linux or POSIX compatible?

No. Starina provides completely original APIs and fresh new development experiences. However, to make it easier to adapt to Starina, We plan to implement a [WSL2-like](https://learn.microsoft.com/en-us/windows/wsl/about#what-is-wsl-2) seamless Linux environment based on real Linux microVM + lightweight integration layer (akin to [LWK](https://en.wikipedia.org/wiki/Lightweight_kernel_operating_system) in [supercomputing](https://link.springer.com/book/10.1007/978-981-13-6624-6)).

## Why Rust?

We (and perhaps you too) love to debate the best text editor and programming language, sometimes very seriously and passionately.

Starina is entirely written in Rust because it is *"C++ with seatbelts"*, which is suitable for building a robust yet high-performance OS. Seatbelts are sometimes annoying indeed, but we know it saved us from countless bugs by enforcing good practices. Notably, I don't need address sanitizer when writing Rust. That's a huge factor for me.

That said, it's crystal clear that Rust (or any other language) is not the best language for everything. That's why Starina is designed to be language-agnostic, and I plan to add seamless support for other languages such as TypeScript. What if you can prototype OS components such as device drivers, as if you are writing a Web app? Isn't that cool?
