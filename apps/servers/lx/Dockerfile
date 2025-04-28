# Builds a minimal Linux distro for Starina on Linux.
#
# Tip: How to edit the kernel config:
#
#   docker build -t lx-kernel-builder --target kernel-builder -f Dockerfile .
#   docker run -v $PWD:/lx -it lx-kernel-builder bash -c "make -C /linux menuconfig && cp /linux/.config /lx/linux.riscv64.config"
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
# ADD linux.riscv64.config .config
# RUN make olddefconfig
# RUN make -j$(nproc)

FROM rust:1.86 AS lxinit-builder
ADD lxinit /app
WORKDIR /app

RUN rustup target add riscv64-unknown-linux-gnu
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --target riscv64-unknown-linux-gnu

FROM ubuntu:24.04 AS initramfs-builder
RUN mkdir -p /rootfs
RUN mkdir -p /rootfs/bin
RUN mkdir -p /rootfs/dev
RUN mkdir -p /rootfs/etc
RUN mkdir -p /rootfs/proc
RUN mkdir -p /rootfs/sys
RUN mkdir -p /rootfs/tmp
RUN mkdir -p /rootfs/var
COPY --from=lxinit-builder /app/target/release/lxinit /rootfs/lxinit

FROM ubuntu:24.04
RUN mkdir -p /build
COPY --from=kernel-builder /linux/arch/riscv/boot/Image /build/linux.elf
CMD ["/bin/bash"]
