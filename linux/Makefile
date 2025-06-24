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

vmlinux := kernel/arch/riscv/boot/Image

.PHONY: all
all: $(vmlinux)

kernel:
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

kernel/.config: kernel linux.riscv64.config
	cp linux.riscv64.config kernel/.config
	$(PROGRESS) "make olddefconfig"
	gmake -C kernel olddefconfig

$(vmlinux): kernel kernel/.config
	$(PROGRESS) "make Image"
	gmake -C kernel -j$(nproc) Image
