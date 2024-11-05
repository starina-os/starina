# Starina

Starina ("star" + "-ina", inspired by [stellina](https://en.wiktionary.org/wiki/stellina)) is a new general-purpose operating system based on a microkernel architecture. It is designed to provide the best developer experience, enabling even kernel newbies to easily understand and enjoy developing an OS.

## Website is running on Starina!

Visit **[starina.dev](https://starina.dev)** to experience the very first (trivial) production use of Starina, on Linux/QEMU hypervisor on Raspberry Pi imitating a KVM-based cloud environment.

Your every single request boots a new Starina VM instantly.

## Why Starina?

"What if we try building a microkernel-based general-purpose OS with 21st-century technologies?" This is the question we are trying to answer. There are many microkernel projects out there; however, they often aim to be hobby or research projects or are designed for embedded systems.

Microkernels have been considered impractical in the general-purpose OS world due to IPC overhead, but hardware and software have evolved significantly since the 1990s. Isn't it time to revisit the microkernel architecture? Let's see how far we can go with ideas we have today!

Starina aims to be:

- **Simple:** Easy to understand and develop, even for non-experts.
- **Practical:** Aiming to be a general-purpose operating system, not just a hobby or research project.
- **Performant:** Not sticking rigidly to a beautiful design; compromising for performance when necessary (without sacrificing microkernel principles of course).

## Design Principles

To achieve this goal, we have the following design principles:

- Aim to be easy to develop, not to achieve a correct and elegant architecture. Make OS development approachable and fun for everyone.
- Don't try to achieve the perfect design from the beginning. Imagine how the userspace should look first, not vice versa - the microkernel is just a runtime for applications.
- The traditional "user-mode" concept is just one of many ways to isolate OS components. Implement faster alternatives like language-based isolation (e.g., Rust/WebAssembly) and Intel/Arm-specific mechanisms (e.g., Intel PKS) for better performance.
- Implement in [Rust](https://www.rust-lang.org/) with async APIs, without using async Rust (`async fn`). Every component has a simple main loop to make the execution flow clear.

## Is it Linux or POSIX compatible?

No. Starina provides completely original APIs and fresh new development experiences. However, to make it easier to adapt to Starina, I plan to implement seamless Linux environment based on real Linux microVM + lightweight integration layer (akin to [LWK](https://en.wikipedia.org/wiki/Lightweight_kernel_operating_system) in [supercomputing](https://link.springer.com/book/10.1007/978-981-13-6624-6)).

## Features

- x86_64, 64-bit Arm (AArch64), and 64-bit RISC-V support.
- A new microkernel written in Rust.
- Multiple component isolation modes: in-kernel (trust [Rust's safety](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)), user mode, and in the future, WebAssembly.
- Rust API and libraries for applications.
- [smoltcp](https://github.com/smoltcp-rs/smoltcp)-based TCP/IP stack.
- Virtio device drivers to support cloud environments.
- Intuitive Rust API for apps, OS servers, and device drivers.
- Auto-generated IPC stubs and startup code from declarative spec files.

## Getting Started

See the [Quickstart](docs/quickstart.md) guide to get started with Starina.

## License

Starina is dual-licensed under the [MIT license](https://opensource.org/license/mit) and the [Apache 2.0 license](https://opensource.org/license/apache-2-0).
