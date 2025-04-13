#!/bin/bash
set -xue

clang \
    -Oz -flto=thin -fomit-frame-pointer -fmerge-all-constants \
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
wasm-opt -Oz app.stage1.wasm -o app.stage2.wasm
llvm-strip app.stage2.wasm -o app.optimized.wasm

rm app.stage0.wasm app.stage1.wasm app.stage2.wasm
