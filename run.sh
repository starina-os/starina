#!/bin/bash
set -eu

export STARINA_ARCH="${STARINA_ARCH:-riscv64}"
export LINUXRUN_IMAGE="${LINUXRUN_IMAGE:-docker://hello-world:latest}"
export LINUXRUN_ENTRYPOINT="${LINUXRUN_ENTRYPOINT:-/hello}"
export LINUXRUN_ARCH="${STARINA_ARCH}"

# V=1
if [[ -n ${V:-} ]]; then
  set -x
fi

QEMU=${QEMU:-qemu-system-riscv64}

cd "$(dirname "$0")"

export CARGO_TARGET_DIR=build
export CARGO_TERM_HYPERLINKS=false
export STARINA_RUN_SH=1

cargo_cmd=build
if [[ -n ${CHECK_ONLY:-} ]]; then
    cargo_cmd=check
elif [[ -n ${CLIPPY:-} ]]; then
    cargo_cmd="clippy --fix --allow-staged --allow-dirty"
fi

cargo $cargo_cmd \
    ${V:+-vvv} \
    ${RELEASE:+--release} \
    ${EXIT_ON_IDLE:+--features exit-on-idle} \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --target kernel/src/arch/${STARINA_ARCH}/kernel.json \
    --manifest-path kernel/Cargo.toml

if [[ -n ${CHECK_ONLY:-} || -n ${CLIPPY:-} ]]; then
    exit 0
fi

if [[ -n ${RELEASE:-} ]]; then
    cp build/kernel/release/kernel starina.elf
else
    cp build/kernel/debug/kernel starina.elf
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
    -netdev user,id=net0,hostfwd=tcp:127.0.0.1:30080-:80,hostfwd=tcp:127.0.0.1:38080-:8080 \
    -d cpu_reset,unimp,guest_errors,int -D qemu.log \
    -gdb tcp::7778 ${WAIT_FOR_GDB:+-S}
