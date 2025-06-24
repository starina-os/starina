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

image_path := arch/riscv/boot/Image

ifeq ($(shell uname),Darwin)
    gnumake_dir := $(shell brew --prefix make)
    linux_make := $(gnumake_dir)/bin/gmake

    findutils_dir := $(shell brew --prefix findutils)
    export PATH := $(findutils_dir)/libexec/gnubin:$(PATH)

    llvm_prefix := $(shell brew --prefix llvm)/bin
    libelf_dir := $(shell brew --prefix libelf)
    export HOSTCFLAGS=-Iscripts/macos-include -I $(libelf_dir)/include
    # Suppress "cpio: Couldn't list extended attributes" warning.
    export COPYFILE_DISABLE=1
else ifeq ($(shell uname),Linux)
    linux_make := make
    # Add snap binaries to PATH for zig
    export PATH := /snap/bin:$(PATH)
else
    $(error "Unsupported platform: $(shell uname)")
endif

export ARCH=riscv
export LLVM=1
export LD=$(llvm_prefix)ld.lld
export HOSTCC=$(llvm_prefix)clang
export CC=$(llvm_prefix)clang
export NM=$(llvm_prefix)nm
export OBJCOPY=$(llvm_prefix)llvm-objcopy
export OBJDUMP=$(llvm_prefix)llvm-objdump

.PHONY: all
all: linux.elf

linux.elf: $(image_path)
	cp kernel/$(image_path) linux.elf

kernel/README:
	$(PROGRESS) "Downloading Linux kernel..."
	curl -fSL -o linux.tar.xz.tmp https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.12.34.tar.xz
	mv linux.tar.xz.tmp linux.tar.xz
	$(PROGRESS) "Extracting Linux kernel..."
	mkdir -p kernel.tmp
	tar xf linux.tar.xz -C kernel.tmp --strip-components=1
	$(PROGRESS) "Applying building-linux-on-macos.patch"
	cd kernel.tmp && patch -p1 < ../building-linux-on-macos.patch
	mv kernel.tmp kernel

kernel/.config: kernel/README linux.riscv64.config
	cp linux.riscv64.config kernel/.config
	$(PROGRESS) "make olddefconfig"
	$(linux_make) -C kernel olddefconfig

initramfs_files := build/linuxinit build/catsay

$(image_path): kernel/.config initramfs.list $(initramfs_files)
	$(PROGRESS) "make $(image_path)"
	$(linux_make) -C kernel Image V=1

build/catsay: $(wildcard catsay.go)
	mkdir -p build
	CGO_ENABLED=0 GOARCH=riscv64 GOOS=linux go build -o build/catsay catsay.go

build/linuxinit: $(wildcard linuxinit/src/*.rs linuxinit/Cargo.toml)
	mkdir -p build
	cd linuxinit && RUSTFLAGS="-C linker=./zig-linker.sh" cargo build --release --target riscv64gc-unknown-linux-musl -vvv
	cp linuxinit/target/riscv64gc-unknown-linux-musl/release/linuxinit $(@)
