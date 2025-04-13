#!/bin/bash
set -xue

clang \
    -Oz -flto=thin -fomit-frame-pointer -fmerge-all-constants \
    -mbulk-memory \
    --target=wasm32-wasi \
    --sysroot=/opt/homebrew/opt/wasi-libc/share/wasi-sysroot \
    -nodefaultlibs \
    -L libclang_rt.builtins-wasm32-wasi-24.0/ \
    -DQJS_BUILD_LIBC \
    -D_WASI_EMULATED_SIGNAL \
    -D_GNU_SOURCE \
    -lwasi-emulated-signal \
    -lc \
    -lclang_rt.builtins-wasm32 \
    quickjs-amalgam.c main.c \
    -o app.stage0.wasm

wizer --allow-wasi app.stage0.wasm -o app.stage1.wasm
wasm-opt -Oz --enable-bulk-memory-opt --strip-debug --strip-dwarf app.stage1.wasm -o app.wasm
ls -lh app.wasm

rm app.stage0.wasm app.stage1.wasm
