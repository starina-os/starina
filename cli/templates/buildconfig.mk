#
#  Build configuration for make command.
#

# CPU architecture and machine type:
#
#   ARCH=riscv64 MACHINE=qemu-virt
#   ARCH=arm64   MACHINE=qemu-virt
#   ARCH=x64     MACHINE=pc
#
ARCH=riscv64
MACHINE=qemu-virt

# Apps to build.
APPS = apps/echo apps/tcpip apps/virtio_net apps/http_server

# Apps to be automatically started by the kernel.
STARTUP_APPS = $(APPS)

# 1 to enable release build.
RELEASE =
