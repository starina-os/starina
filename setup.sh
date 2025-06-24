#!/bin/sh
set -ue

setup_macos() {
    set -x
    brew install make llvm zig findutils libelf
}

setup_linux() {
    set -x
    sudo apt-get install \
        build-essential clang llvm \
        curl flex bison bc cpio lz4 libelf-dev
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
