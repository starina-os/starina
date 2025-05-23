#!/bin/bash
set -eu

# V=1
if [[ -n ${V:-} ]]; then
  set -x
fi

QEMU=${QEMU:-qemu-system-riscv64}

cd "$(dirname "$0")"

export CARGO_TARGET_DIR=build
export CARGO_TERM_HYPERLINKS=false

cargo_cmd=build
if [[ -n ${CHECK_ONLY:-} ]]; then
    cargo_cmd=check
fi

cargo $cargo_cmd \
    ${V:+-v} \
    ${RELEASE:+--release} \
    ${EXIT_ON_IDLE:+--features exit-on-idle} \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --target kernel/src/arch/riscv64/kernel.json \
    --manifest-path kernel/Cargo.toml

if [[ -n ${CHECK_ONLY:-} ]]; then
  exit 0
fi

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
$QEMU -machine virt -cpu rv64,h=true,sstc=true -m 256 -bios default \
  -kernel starina.elf \
  -semihosting \
  -nographic -serial mon:stdio --no-reboot \
  -global virtio-mmio.force-legacy=false \
  -device virtio-net-device,netdev=net0,bus=virtio-mmio-bus.0 \
  -object filter-dump,id=fiter0,netdev=net0,file=virtio-net.pcap \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:1234-:80 \
  -d cpu_reset,unimp,guest_errors,int -D qemu.log \
  -gdb tcp::7778 ${WAIT_FOR_GDB:+-S}
