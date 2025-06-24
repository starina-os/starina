# The default build target.
.PHONY: default
default: all

# Disable builtin implicit rules and variables.
MAKEFLAGS += --no-builtin-rules --no-builtin-variables
.SUFFIXES:

# Enable verbose output if $(V) is set.
ifeq ($(V),)
.SILENT:
endif

PROGRESS := printf "\\033[1;96m==>\\033[0m \\033[1m%s\\033[0m\\n"

export ARCH=riscv
export LLVM=1
export LD=$(LLVM_PREFIX)ld.lld
export CC=$(LLVM_PREFIX)clang
export NM=$(LLVM_PREFIX)nm
export OBJCOPY=$(LLVM_PREFIX)llvm-objcopy
export OBJDUMP=$(LLVM_PREFIX)llvm-objdump

ifeq ($(shell uname),Darwin)
    LLVM_PREFIX := $(shell brew --prefix llvm)/bin
    LIBELF_DIR := $(shell brew --prefix libelf)
    export HOSTCFLAGS=-Iscripts/macos-include -I $(LIBELF_DIR)/include
else
    $(error "Unsupported platform: $(shell uname)")
endif

image_path := arch/riscv/boot/Image

.PHONY: all
all: linux.elf

linux.elf: $(image_path)
	cp kernel/$(image_path) linux.elf

kernel/README:
	$(PROGRESS) "Extracting Linux kernel..."
	mkdir -p kernel.tmp
	tar xf linux.tar.xz -C kernel.tmp --strip-components=1
	$(PROGRESS) "Applying building-linux-on-macos.patch"
	cd kernel.tmp && patch -p1 < ../building-linux-on-macos.patch
	mv kernel.tmp kernel

linux.tar.xz:
	$(PROGRESS) "Downloading Linux kernel..."
	curl -fSL -o linux.tar.xz.tmp https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.12.34.tar.xz
	mv linux.tar.xz.tmp linux.tar.xz

kernel/.config: kernel/README linux.riscv64.config
	cp linux.riscv64.config kernel/.config
	$(PROGRESS) "make olddefconfig"
	gmake -C kernel olddefconfig

$(image_path): kernel/.config initramfs.cpio
	$(PROGRESS) "make $(image_path)"
	gmake -C kernel Image

initramfs.cpio: initramfs/linuxinit initramfs/bin/catsay initramfs/etc/resolv.conf initramfs/dev/console
	mkdir -p initramfs/bin
	mkdir -p initramfs/etc
	mkdir -p initramfs/dev
	mkdir -p initramfs/proc
	mkdir -p initramfs/sys
	mkdir -p initramfs/tmp
	mkdir -p initramfs/var
	mkdir -p initramfs/mnt
	mkdir -p initramfs/virtfs
	find initramfs -print0 | cpio --null -o --format=newc > initramfs.cpio

initramfs/etc/resolv.conf:
	mkdir -p initramfs/etc
	ln -sf /proc/net/pnp initramfs/etc/resolv.conf

initramfs/bin/catsay:
	mkdir -p initramfs/bin
	CGO_ENABLED=0 GOARCH=riscv64 GOOS=linux go build -o initramfs/bin/catsay catsay.go

initramfs/dev/console:
	mkdir -p initramfs/dev
	sudo mknod initramfs/dev/console c 5 1

initramfs/linuxinit: $(wildcard linuxinit/src/*.rs linuxinit/Cargo.toml)
	mkdir -p initramfs
	cd linuxinit && RUSTFLAGS="-C linker=./zig-linker.sh" cargo build --release --target riscv64gc-unknown-linux-musl
	cp linuxinit/target/riscv64gc-unknown-linux-musl/release/linuxinit $(@)
