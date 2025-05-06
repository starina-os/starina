# Builds a minimal Linux distro for Starina on Linux.
#
#   docker build -t lx-builder -f Dockerfile .
#   docker run -v $PWD:/lx -it lx-builder cp /build/linux.elf /lx/linux.elf
#

#
#  init builder
#
FROM rust:1.86 AS lxinit-builder

RUN rustup target add riscv64gc-unknown-linux-musl

RUN apt-get update && apt-get install -y \
    gcc-riscv64-linux-gnu \
    binutils-riscv64-linux-gnu

RUN mkdir -p /app
ADD lxinit /app
WORKDIR /app

ENV CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_MUSL_LINKER="riscv64-linux-gnu-gcc"
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=z
ENV CARGO_PROFILE_RELEASE_DEBUG=full
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --target riscv64gc-unknown-linux-musl \
    && cp /app/target/riscv64gc-unknown-linux-musl/release/lxinit /lxinit.debug \
    && cp /app/target/riscv64gc-unknown-linux-musl/release/lxinit /lxinit \
    && riscv64-linux-gnu-strip /lxinit

#
#  initramfs builder
#
FROM ubuntu:24.04 AS initramfs-builder

RUN apt-get update && apt-get install -y \
    cpio

RUN mkdir -p /rootfs
RUN mkdir -p /rootfs/bin
RUN mkdir -p /rootfs/dev
RUN mkdir -p /rootfs/etc
RUN mkdir -p /rootfs/proc
RUN mkdir -p /rootfs/sys
RUN mkdir -p /rootfs/tmp
RUN mkdir -p /rootfs/var
RUN mkdir -p /rootfs/virtfs

RUN mknod /rootfs/dev/console c 5 1

COPY --from=lxinit-builder /lxinit /rootfs/init

WORKDIR /rootfs
RUN find . -print0 | cpio --null -o --format=newc > /initramfs.cpio

#
#  Kernel builder
#
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libncurses-dev \
    flex \
    bison \
    bc \
    cpio \
    lz4 \
    libelf-dev \
    gcc-riscv64-linux-gnu \
    binutils-riscv64-linux-gnu

RUN apt-get install -y llvm
RUN apt-get install -y less

COPY --from=initramfs-builder /initramfs.cpio /initramfs.cpio
COPY --from=lxinit-builder /lxinit.debug /lxinit.debug
COPY --from=lxinit-builder /lxinit /lxinit

CMD make -C /lx/linux ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- -j$(nproc) \
        olddefconfig Image
