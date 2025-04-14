# Porting

Starina is designed to be highly portable. Arch-dependent code is clearly separated in the codebase, supports Device Tree, and leverages Cargo's build system to cross-compile OS.

## Porting to a New Board

If Starina already supports the ISA (e.g. `riscv64`), it would be easy to port. You'll need to change:

- Kernel entry point. For example, `riscv64_boot` expects hart ID in `a0`, and device tree blob in `a1` register on entry. Your board's bootloader may have a different protocol when entering the kernel.
- The base address in `kernel.ld` to match the board's memory map.
- Kernel memory areas mapped in `kernel::arch::VmSpace::map_kernel_space`.

> [!TIP]
>
> Starina is still premature, and it does not have a fancy build config system like Linux's Kconfig. Try making it work first, hard-coded things everywhere, and then open a PR to discuss how we can generalize it.

## Porting to a New ISA (CPU architecture)

You'll need to implement the following files:

- Install Rust toolchain
- `kernel/arch/<arch>`: Arch-dependent code in the kernel. `kernel/arch/host` is a good skeleton.
- `run.sh`: Change `--target` parameter for `cargo build`, and `$QEMU` for `qemu`.

> [!TIP]
>
> To generate `kernel.json` template, you can use `rustc`'s `--print target-spec-json` feature ([documentation](https://doc.rust-lang.org/rustc/targets/custom.html)):
>
> ```bash
> # Look for a suitable target to use
> rustc --print target-list | grep x86_64
>
> # Print a template target JSON for x86_64-unknown-none
> rustc +nightly -Z unstable-options \
>   --target=x86_64-unknown-none --print target-spec-json
> ```
