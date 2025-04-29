# Builds a minimal Linux distro for Starina on Linux.
#
#   docker build -t lx-builder -f Dockerfile .
#   docker run -v $PWD:/lx -it lx-builder cp /build/linux.elf /lx/linux.elf
#

#
#  init builder
#
FROM rust:1.86 AS lxinit-builder

RUN mkdir -p /app
ADD lxinit /app
WORKDIR /app

RUN rustup target add riscv64gc-unknown-linux-musl

RUN apt-get update && apt-get install -y \
    gcc-riscv64-linux-gnu \
    binutils-riscv64-linux-gnu

ENV CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_MUSL_LINKER="riscv64-linux-gnu-gcc"
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=z
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --target riscv64gc-unknown-linux-musl \
    && cp /app/target/riscv64gc-unknown-linux-musl/release/lxinit /lxinit \
    && riscv64-linux-gnu-strip /lxinit

#
#  initramfs builder
#
FROM ubuntu:24.04 AS initramfs-builder

RUN apt-get update && apt-get install -y \
    cpio \
    lz4

RUN mkdir -p /rootfs
RUN mkdir -p /rootfs/bin
RUN mkdir -p /rootfs/dev
RUN mkdir -p /rootfs/etc
RUN mkdir -p /rootfs/proc
RUN mkdir -p /rootfs/sys
RUN mkdir -p /rootfs/tmp
RUN mkdir -p /rootfs/var
COPY --from=lxinit-builder /lxinit /rootfs/lxinit

WORKDIR /rootfs
RUN find . -print0 | cpio --null -o --format=newc | lz4 -9 > /initramfs.cpio.lz4

#
#  Kernel builder
#
FROM ubuntu:24.04 AS kernel-builder

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libncurses-dev \
    flex \
    bison \
    libelf-dev

RUN curl -fSL -o linux.tar.xz https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.12.25.tar.xz
RUN mkdir -p linux && tar -xf linux.tar.xz -C linux --strip-components=1
WORKDIR /linux

RUN apt-get update && apt-get install -y \
    gcc-riscv64-linux-gnu \
    binutils-riscv64-linux-gnu \
    bc \
    cpio

ADD linux.riscv64.config .config
COPY --from=initramfs-builder /initramfs.cpio.lz4 /initramfs.cpio.lz4

RUN make ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- -j$(nproc) \
        olddefconfig Image

RUN cp /linux/vmlinux /linux.elf
RUN cp /linux/arch/riscv/boot/Image /linux.bin

#
#  Final stage
#
FROM ubuntu:24.04
RUN mkdir -p /build
COPY --from=kernel-builder /linux.bin /build/linux.bin
COPY --from=kernel-builder /linux.elf /build/linux.elf
CMD ["/bin/bash"]
