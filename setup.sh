#!/bin/sh
set -ue

setup_macos() {
    set -x
    brew install make llvm lld zig findutils libelf
}

setup_linux() {
    set -x
    sudo apt-get update
    sudo apt-get install -y \
        build-essential clang llvm \
        curl flex bison bc cpio lz4 libelf-dev \
        golang-go lld
}

common_setup() {
    if ! command -v rustup &> /dev/null; then
        echo "rustup is not installed - visit https://rustup.rs" >&2
        exit 1
    fi

    set -x
    rustup target add riscv64gc-unknown-linux-musl
}

case "$(uname)" in
    Darwin)
        setup_macos
        ;;
    Linux)
        setup_linux
        ;;
    *)
        echo "Unsupported platform: $(uname)"
        exit 1
        ;;
esac
