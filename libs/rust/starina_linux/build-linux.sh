#!/bin/bash
set -eu

if [[ ! -d /linux/kernel ]]; then
    echo "Downloading Linux kernel..."
    curl -fSL -o linux.tar.xz.tmp https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.12.25.tar.xz
    mv linux.tar.xz.tmp linux.tar.xz
    mkdir -p /linux/kernel
    tar xf linux.tar.xz -C /linux/kernel --strip-components=1
fi

cp /linux.config /linux/kernel/.config
make -C /linux/kernel ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- -j$(nproc) \
    olddefconfig Image
