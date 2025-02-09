# Starina

Starina (named after [stellina](https://en.wiktionary.org/wiki/stellina)), is a general-purpose, microkernel-based, modern operating system designed for developers.

## Goals

The utltimate goal of this project is to create a production-ready OS to be a good alternative to real-world OSes. To make this happen, Starina values the following principles:

- **Userspace-first approach:** Prioritize developer experience in the userspace, where the most OS components reside. The microkernel is just a runtime for applications. Make OS development approachable and fun for everyone. 
- **Simplicity over perfection:** Emphasize a straightforward design which covers the most common use cases. Make it work first, then make it better.
- **Incrementally adoptable:** Facilitate easy adoption of Starina by providing a seamless integration with existing systems.

## Roadmap

- [x] Prototyping an microkernel-based OS in Rust ([https://starina.dev](https://starina.dev) is served by Starina on Linux/QEMU hypervisor!).
- [ ] Redesign the OS based on lessons learned (work in progress to be done by Feb).
- [ ] WSL2-like but faster and more seamless Linux compatibility layer.
- [ ] Make it production-ready.

## Is it Linux or POSIX compatible?

No. Starina provides completely original APIs and fresh new development experiences. However, to make it easier to adapt to Starina, We plan to implement a [WSL2-like](https://learn.microsoft.com/en-us/windows/wsl/about#what-is-wsl-2) seamless Linux environment based on real Linux microVM + lightweight integration layer (akin to [LWK](https://en.wikipedia.org/wiki/Lightweight_kernel_operating_system) in [supercomputing](https://link.springer.com/book/10.1007/978-981-13-6624-6)).
