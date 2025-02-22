#!/bin/bash
set -eu

QEMU=${QEMU:-qemu-system-riscv64}

export CARGO_TERM_HYPERLINKS=false
cargo build \
  -Z build-std=core,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --target kernel/arch/riscv64/kernel.json \
  --manifest-path kernel/Cargo.toml

$QEMU -machine virt -m 256 -bios default \
  -kernel target/kernel/debug/kernel \
  -nographic -serial mon:stdio --no-reboot \
  -d cpu_reset,unimp,guest_errors,int -D qemu.log
