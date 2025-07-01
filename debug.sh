#!/bin/bash
set -eu

cd "$(dirname "$0")"

echo "*** Attaching GDB to QEMU..."
riscv64-elf-gdb -q
