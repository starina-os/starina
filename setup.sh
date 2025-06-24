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

    sudo snap install zig --classic --beta
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
