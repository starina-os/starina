# Comparison with Others

*"How's Starina different from X?"* is the question you'll probably ask first. In this article, we will explore the unique features, Design choices, advantages, and most importantly, the disadvantages of Starina compared to other microkernels.

This kind of article is uncomfortable to write because some use this kind of comparison for a marketing battle, or FUD. Thus, I'd make it clear that ***"it depends"***. Differences are why there are many microkernels, text editors, programming languages, and Ramen restaurants in this world.

If you found something wrong or inaccurate, please open an issue or a pull request. I will be happy to correct it :)

## Userspace-First Design

This is vague but the most important philosophy of Starina. Starina is designed to be a userspace-first microkernel, which means we design how we want to write applications and OS components in userspace, instead of achieving the most ideal kernel design.

This means the kernel may sometimes have some nasty hacks to make things work for now. For example, the current kernel does dynamic memory allocation in the kernel, which makes it more monolithic than other strict microkernels. However, until we really need to do that, we prefer to keep it intuitive for newbies.

The opposite of this is what I call "kernel-first" design. seL4 is a good example of this. seL4 is an extremely strict design. I'm saying *strict* not because it's formally verified, but because [its API](https://docs.sel4.systems/projects/sel4/api-doc.html) is super minimal. You may notice that it exposes low-level hardware details directly (e.g. `seL4_X86_PageTable` and `seL4_ARM_PageTable`) and has no dynamic memory allocation API. This lack of abstractions makes the kernel minimal, and gives you the freedom to implement your own abstractions.

## Multiple Process Isolation Mechanisms

### A Little Bit of Background (for Microkernel Newbies)

Microkernel is a Design pattern where the kernel is as small as possible, and everything else is implemented as user-space processes. In so-called multi-server microkernels, the userland OS components are implemented as separate processes. For example, TCP/IP process, file system, and each device driver have their own process.

Separate processes here means that they are isolated from each other, as in they cannot access each other's memory nor other kernel resources (e.g. file descriptors). This makes the system more secure and stable, as a bug in one process cannot crash the whole system. This is called "process isolation", and is a key feature of microkernels.

Traditionally, process isolation is achieved by virtual memory, aka. paging. Each process has its own virtual address space, and the CPU enforces this isolation. OS components communicate with each other using IPC (Inter-Process Communication) mechanisms, such as message passing and shared memory. Since monolithic kernels do function calls instead of IPC, it's intuitive to think that microkernels are slower than monolithic kernels due to IPC overheads.

### Starina's Approach

In Starina, process isolation can be done in different ways, depending on your needs. Currently Starina *plans to* support:

- **In-kernel Rust-based isolation:** Trust [safe Rust](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html) code to be memory safe and use Rust's type system to enforce isolation. This enables super lightweight processes as they're embedded in the kernel. Good enough isolation for trusted components.
- **In-kernel WebAssembly-based isolation (work-in-progress):** Use in-kernel WebAssembly engine to guarantee memory safety and isolation. This is a good option for untrusted components, and is also nice for porting existing [WASI-based](https://wasi.dev/) applications.
- **Usermode isolation (planned):** Traditional usermode + page table isolation. This is currently not implemented simply because this is not prioritized for now. However, Starina is designed to support this easily in the future, and in-kernel Rust apps would be able to run in usermode transparently as well, without any code changes.
- **VMM-based isolation (planned):** Run untrusted and potentially malicious code in another hardware-based isolation mechanism: Intel VT-x or Arm virtualization extensions. While it is famous for running virtual machines, we run apps in the isolation directly. This is similar to [Hyperlight](https://opensource.microsoft.com/blog/2024/11/07/introducing-hyperlight-virtual-machine-based-security-for-functions-at-scale/).

Why multiple isolation mechanisms? Because it always depends on the use case. For example, you can trust core components like the official TCP/IP server and run it in kernel space for performance, while running device drivers written in C in usermode for reliability, and eventually run untrusted potentially-malicious code in VMM-based isolation for security in the future.

## Message Passing without IDL

Message passing is a major IPC mechanism in microkernels. It's similar to UNIX domain socket but in a datagram-like way.

Typical microkernels (and so does Starina) do not parse the message contents, but treat it as an opaque byte array. This means that the sender and receiver must agree on the message format, which is usually done using an Interface Definition Language (IDL). For example, Fuchsia uses its own IDL called [FIDL](https://fuchsia.dev/fuchsia-src/concepts/fidl/overview).

### Starina's Approach

Starina uses message passing for IPC, without IDL. Instead, it has a predefined set of message types. This sounds like moving backwards, but it actually has some advantages:

- **No new language to learn:** You don't need to learn a new IDL language. Just learn the few predefined message types.
- **No code generation:** IPC stubs are not necessary. This makes the code simpler and easier to read and debug.
- **Speed:** We can optimize the message passing for the predefined message types.
- **Composability:** You'll be able to compose apps like piping UNIX commands (`cat | grep | wc`) thanks to the uniform interface.

To summarize, Starina has *"everything is a file"*-like philosophy in message passing. That is, we prefer a simple interface which covers 90% of the use cases, instead of having specialized interfaces for each use case. A key my finding here is that interactions between OS components are way simpler than gRPC-powered applications.

## Declarative Initialization

Starina prefers declarative ways to initialize components. This is similar to Kubernetes: you write YAML files to describe the desired state of the system, and Kubernetes takes care of the rest. Starina does the same thing, but in the operating system level.

In Starina, apps describe required resources in a spec file such as channels to servers, device tree, and config parameters, and so on. The startup process (currently implemented in kernel) reads the spec files, prepares requested resources, and starts the app with environment variables.

Apps start with environment variables that are set by the startup process. No command-line parsing nor service discovery is needed. This makes the app initialization much simpler, less boilerplate, and more consistent.

## Embrace LLMs in OS Development

TODO: Stay tuned ;)

<!-- Apps should look similar, like React apps, Rails, ... -->

## Lightweight VM for Linux Compatibility

TODO: Planned to be done in next vacation
