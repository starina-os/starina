# Kernel Coding Guidelines

Kernel development is much more strict than application development. In this guide, we present some implicit rules that you should follow when writing kernel code.

## Single kernel stack design

Unlike traditional operating systems, Starina kernel uses a single stack per CPU, instead of having a dedicated stack for each thread. This design resembles how async Rust works - we need a separate state machine. In kernel, we don't use `async`/`await` syntax, but we use `ThreadState` to represent the state of a thread.

## APIs

| std equivalent | kernel alternative | Remarks |
|----------------|--------------------|----|
| `HashMap` | `crate::utils::ConstHashMap` | If you want to intiialize a `HashMap` in a `const fn`. |
| `Arc` | `crate::refcount::SharedRef` |

## Rules

- Avoid memory allocation panics. Use `try_reserve` before adding a new element to a collection such as a `Vec`.
