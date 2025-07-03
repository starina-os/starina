#!/bin/sh
exec zig cc -target riscv64-linux-musl "$@"
