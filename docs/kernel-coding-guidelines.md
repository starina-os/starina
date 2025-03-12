# Kernel Coding Guidelines

Kernel development is much more strict than application development. In this guide, we present some implicit rules that you should follow when writing kernel code.

## Execution flow

Once the kernel is booted, it will behaves like an event handler: it waits for events (e.g. system calls, interrupts, and exceptions), saves the current thread's state, does the necessary job, and resumes a thread.

## Single kernel stack design

Unlike traditional operating systems, Starina kernel uses a single stack per CPU, instead of having a dedicated stack for each thread. This design resembles how async Rust works - we need a separate state machine. In kernel, we don't use `async`/`await` syntax, but we use `ThreadState` to represent the state of a thread.

## APIs

| `libstd` equivalent | Kernel alternative | Remarks |
|----------------|--------------------|----|
| `HashMap` | `crate::utils::FxHashMap` | If you want to intiialize a `HashMap` in a `const fn`. |
| `Arc` | `crate::refcount::SharedRef` | |
| `Mutex` | `crate::spinlock::SpinLock` | |
| `thread_local` | `CpuVar` | A CPU-local variable, which is similar to `thread_local` in the userspace. |

## Rules

- Avoid `panic`s. If you use `unwrap`, describe why you think it never fails.
- Handle allocation failures in collections (e.g. `Vec`). Use `try_reserve` before adding a new element to a collection.

