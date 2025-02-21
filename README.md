# Starina

Starina (named after [stellina](https://en.wiktionary.org/wiki/stellina)), is a general-purpose, microkernel-based, modern operating system designed for developers. It aims to be a production-ready OS, and more importantly, a fun and easy-to-understand OS where you can enjoy the development as if you are writing a Web application.

> [!NOTE]
>
> This branch is for next generation of Starina, which is being written from scratch. The previous prototype version can be found at [this commit](https://github.com/starina-os/starina/commit/7893df4e218a23b91907f4f9cd238a6f8f5548ee).

## Goals

The ultimate goal of this project is to create a production-ready OS to be a good alternative to real-world OSes. To make this happen, Starina values the following principles:

- **Userspace-first approach:** Make OS development approachable and fun for everyone. Prioritize developer experience in the userspace, where the most OS components reside. The microkernel is just a runtime for applications.
- **Simplicity over perfection:** Emphasize a straightforward design which covers the most common use cases. Make it work first. Make it better later.
- **Incrementally adoptable:** Facilitate easy adoption of Starina by providing a seamless integration with existing systems.

## Roadmap for 2025

This year, we focus on cloud computing domain, where Starina will be used as a tiny runtime for Linux containers.

- [x] Prototyping an microkernel-based OS in Rust: **[https://starina.dev](https://starina.dev) is served by Starina on Linux/QEMU hypervisor!**
- [x] Redesign the OS based on lessons learned
- [ ] Rewrite from scratch (work in progress)
- [ ] Rust-based almost-zero-cost isolation ([Unikernel](https://en.wikipedia.org/wiki/Unikernel) style)
- [ ] Usermode isolation (traditional microkernel style)
- [ ] TCP/IP server
- [ ] File system server
- [ ] TypeScript API + QuickJS-based isolation (akin to WebAssembly)
- [ ] Shell
- [ ] WSL2-like Linux compatibility layer
- [ ] Streamlined observability and debugging experience

## Is it Linux or POSIX compatible?

No. Starina provides completely original APIs and fresh new development experiences. However, to make it easier to adapt to Starina, We plan to implement a [WSL2-like](https://learn.microsoft.com/en-us/windows/wsl/about#what-is-wsl-2) seamless Linux environment based on real Linux microVM + lightweight integration layer (akin to [LWK](https://en.wikipedia.org/wiki/Lightweight_kernel_operating_system) in [supercomputing](https://link.springer.com/book/10.1007/978-981-13-6624-6)).
