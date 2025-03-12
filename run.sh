#!/bin/bash
set -eu

QEMU=${QEMU:-qemu-system-riscv64}

export CARGO_TERM_HYPERLINKS=false
cargo build \
  ${RELEASE:+--release} \
  -Z build-std=core,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --target kernel/arch/riscv64/kernel.json \
  --manifest-path kernel/Cargo.toml

if [[ -n ${RELEASE:-} ]]; then
  cp target/kernel/release/kernel starina.elf
else
  cp target/kernel/debug/kernel starina.elf
fi

echo -e "\nStarting QEMU..."
$QEMU -machine virt -m 256 -bios default \
  -kernel starina.elf \
  -nographic -serial mon:stdio --no-reboot \
  -d cpu_reset,unimp,guest_errors,int -D qemu.log \
  ${GDB:+-gdb tcp::7778 -S}
