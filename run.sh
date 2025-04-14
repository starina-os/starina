#!/bin/bash
set -eu

QEMU=${QEMU:-qemu-system-riscv64}

cd "$(dirname "$0")"

export CARGO_TARGET_DIR=build
export CARGO_TERM_HYPERLINKS=false
cargo build \
  ${RELEASE:+--release} \
  --features "${FEATURES:-}" \
  -Z build-std=core,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --target kernel/arch/riscv64/kernel.json \
  --manifest-path kernel/Cargo.toml

if [[ -n ${RELEASE:-} ]]; then
  cp build/kernel/release/kernel starina.elf
else
  cp build/kernel/debug/kernel starina.elf
fi


if [[ -n ${REPLAY:-} ]]; then
  RR_MODE=replay
else
  RR_MODE=record
fi

if [[ -n ${BUILD_ONLY:-} ]]; then
  exit 0
fi

echo -e "\nStarting QEMU..."
$QEMU -machine virt -m 256 -bios default \
  -kernel starina.elf \
  -nographic -serial mon:stdio --no-reboot \
  -global virtio-mmio.force-legacy=false \
  -device virtio-net-device,netdev=net0,bus=virtio-mmio-bus.0 \
  -object filter-dump,id=fiter0,netdev=net0,file=virtio-net.pcap \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:1234-:80 \
  -d cpu_reset,unimp,guest_errors,int -D qemu.log \
  -icount shift=auto,rr=${RR_MODE},rrfile=qemu-replay.bin \
  -gdb tcp::7778 ${WAIT_FOR_GDB:+-S}
