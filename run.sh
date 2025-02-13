#!/bin/bash
set -eux

cargo build \
  -Z build-std=core,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --target boot/riscv64/kernel.json \
  --manifest-path boot/riscv64/Cargo.toml

qemu-system-riscv64 -machine virt -m 256 -bios default \
  -kernel target/kernel/debug/boot_riscv64 \
  -nographic -serial mon:stdio --no-reboot \
  -d cpu_reset,unimp,guest_errors,int -D qemu.log
