STARINA_ARCH ?= riscv64
QEMU ?= qemu-system-riscv64

export LINUXRUN_IMAGE ?= docker://hello-world:latest
export LINUXRUN_ENTRYPOINT ?= /hello
export LINUXRUN_ARCH = $(STARINA_ARCH)

export CARGO_TARGET_DIR = build
export CARGO_TERM_HYPERLINKS = false
export STARINA_MAKEFILE = 1

CARGO     ?= cargo
GDB       ?= riscv64-elf-gdb
PROGRESS  ?= printf "  \\033[1;96m%8s\\033[0m  \\033[1;m%s\\033[0m\\n"

CARGOFLAGS += -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
CARGOFLAGS += --target kernel/src/arch/$(STARINA_ARCH)/kernel.json
CARGOFLAGS += --manifest-path kernel/Cargo.toml
CARGOFLAGS += $(if $(V), -vvv)
CARGOFLAGS += $(if $(RELEASE), --release)

QEMUFLAGS += -machine virt -cpu rv64,h=true,sstc=true -m 256 -bios default
QEMUFLAGS += -kernel starina.elf
QEMUFLAGS += -semihosting
QEMUFLAGS += -nographic -serial mon:stdio --no-reboot
QEMUFLAGS += -global virtio-mmio.force-legacy=false
QEMUFLAGS += -device virtio-net-device,netdev=net0,bus=virtio-mmio-bus.0
QEMUFLAGS += -object filter-dump,id=fiter0,netdev=net0,file=virtio-net.pcap
QEMUFLAGS += -netdev user,id=net0,hostfwd=tcp:127.0.0.1:30080-:80,hostfwd=tcp:127.0.0.1:38080-:8080
QEMUFLAGS += -d cpu_reset,unimp,guest_errors,int -D qemu.log
QEMUFLAGS += -gdb tcp::7778
QEMUFLAGS += $(if $(WAIT_FOR_GDB), -S)

MAKEFLAGS += --no-builtin-rules --no-builtin-variables
.SUFFIXES:

ifndef V
.SILENT:
endif

.PHONY: all build check clippy setup debug run clean

all: build

build:
	$(PROGRESS) "CARGO" starina.elf
	$(CARGO) build $(CARGOFLAGS)
	cp build/kernel/$(if $(RELEASE),release,debug)/kernel starina.elf

clippy:
	$(PROGRESS) "CLIPPY"
	$(CARGO) clippy --fix --allow-staged --allow-dirty $(CARGOFLAGS)

run: build
	$(PROGRESS) "QEMU"
	$(QEMU) $(QEMUFLAGS)

debug:
	$(PROGRESS) "GDB"
	$(GDB) -q

clean:
	$(CARGO) clean
	rm -f starina.elf qemu.log virtio-net.pcap
