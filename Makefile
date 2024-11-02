# ARCH=riscv64 MACHINE=qemu-virt
# ARCH=arm64   MACHINE=qemu-virt
# ARCH=x64     MACHINE=pc
ARCH    ?= arm64
MACHINE ?= qemu-virt

RELEASE ?=            # "1" to build release version
V       ?=            # "1" to enable verbose output
USERMODE ?=           # "1" to enable usermode isolation

APPS         ?= apps/tcpip apps/virtio_net apps/http_server
STARTUP_APPS ?= $(APPS)

BUILD_DIR ?= build

# Disable builtin implicit rules and variables.
MAKEFLAGS += --no-builtin-rules --no-builtin-variables
.SUFFIXES:

# Enable verbose output if $(V) is set.
ifeq ($(V),)
.SILENT:
endif

ifeq ($(RELEASE),1)
BUILD := release
CARGOFLAGS += --release
else
BUILD := debug
endif

ifeq ($(ARCH),riscv64)
QEMU      ?= qemu-system-riscv64
QEMUFLAGS += -machine virt -m 256 -bios default
else ifeq ($(ARCH),arm64)
QEMU      ?= qemu-system-aarch64
ifneq ($(HVF),)
QEMUFLAGS += -accel hvf -machine virt,gic-version=2,highmem=off -cpu host -m 256
else
QEMUFLAGS += -machine virt,gic-version=2 -cpu neoverse-n2 -m 256
endif
else ifeq ($(ARCH),x64)
QEMU      ?= qemu-system-x86_64
QEMUFLAGS += -cpu Icelake-Server -m 256 -machine microvm,ioapic2=off,acpi=off
else
$(error "Unknown ARCH: $(ARCH)")
endif

QEMUFLAGS += -rtc base=localtime
QEMUFLAGS += -global virtio-mmio.force-legacy=false
QEMUFLAGS += -device virtio-net-device,netdev=net0,bus=virtio-mmio-bus.0
QEMUFLAGS += -object filter-dump,id=fiter0,netdev=net0,file=virtio-net.pcap
QEMUFLAGS += -netdev user,id=net0,hostfwd=tcp:127.0.0.1:1234-:80

ifneq ($(KVM),)
QEMUFLAGS += -accel kvm
endif

CARGO    ?= cargo
PROGRESS ?= printf "  \\033[1;96m%8s\\033[0m  \\033[1;m%s\\033[0m\\n"
OBJCOPY  ?= $(shell cargo rustc -Z unstable-options --print sysroot)/lib/rustlib/*/bin/llvm-objcopy

RUSTFLAGS += -Z macro-backtrace --emit asm
CARGOFLAGS += -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem

export CARGO_TERM_HYPERLINKS=false

QEMUFLAGS += -nographic -serial mon:stdio --no-reboot
QEMUFLAGS += -d cpu_reset,unimp,guest_errors,int -D qemu.log
QEMUFLAGS += $(if $(GDB),-gdb tcp::7789 -S)

app_elfs := $(foreach app,$(APPS),$(BUILD_DIR)/$(app).elf)
app_sources += \
	$(shell find \
		libs apps spec \
		-name '*.rs' -o -name '*.ld' -o -name '*.S' -o -name '*.S' \
		-o -name '*.json' -o -name '*.yml' -o -name '*.toml' -o -name '*.j2' \
	)
kernel_sources += \
	$(shell find \
		boot/$(ARCH) kernel libs spec \
		-name '*.rs' -o -name '*.ld' -o -name '*.S' -o -name '*.S' \
		-o -name '*.json' -o -name '*.yml' -o -name '*.toml' -o -name '*.j2' \
	)

.DEFAULT_GOAL := default
default: starina.elf

.PHONY: run
run: starina.elf disk.img
	cp starina.elf build/starina.qemu.elf
ifeq ($(ARCH),x64)
	python3 ./tools/make-bootable-on-qemu.py build/starina.qemu.elf
endif
	$(QEMU) $(QEMUFLAGS) -kernel build/starina.qemu.elf

.PHONY: clean
clean:
	rm -rf $(BUILD_DIR)

.PHONY: clippy
clippy:
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) clippy --fix --allow-dirty --allow-staged $(CARGOFLAGS) --manifest-path boot/$(ARCH)/Cargo.toml

.PHONY: fmt
fmt:
	find boot kernel libs apps tools -name '*.rs' | xargs rustup run nightly rustfmt

.PHONY: fix
fix:
	cargo clippy --fix --allow-dirty --allow-staged $(CARGOFLAGS)

.PHONY: docs
docs:
	rm -rf $(BUILD_DIR)/docs
	mkdir -p $(BUILD_DIR)/docs
	$(PROGRESS) "DOCSHIP" "docs"
	docship --indir docs --outdir $(BUILD_DIR)/docs
	$(MAKE) rustdoc
	mv $(BUILD_DIR)/cargo/doc $(BUILD_DIR)/docs/rust


.PHONY: rustdoc
rustdoc:
	$(PROGRESS) "CARGO" "doc"
	BUILD_DIR="$(realpath $(BUILD_DIR))" \
	CARGO_TARGET_DIR="$(BUILD_DIR)/cargo" \
	STARTUP_APP_DIRS="$(foreach app_dir,$(STARTUP_APPS),$(realpath $(app_dir)))" \
		$(CARGO) doc \
			--package starina_api \
			--package starina_driver_utils

disk.img:
	$(PROGRESS) "GEN" "$(@)"
	dd if=/dev/zero of=$(@) bs=1M count=8

starina.elf: $(kernel_sources) $(app_elfs) Makefile Makefile
	$(PROGRESS) "CARGO" "boot/$(ARCH)"
	RUSTFLAGS="$(RUSTFLAGS)" \
	CARGO_TARGET_DIR="$(BUILD_DIR)/cargo" \
	BUILD_DIR="$(realpath $(BUILD_DIR))" \
	$(if $(USERMODE),USERMODE=1) \
	STARTUP_APP_DIRS="$(foreach app_dir,$(STARTUP_APPS),$(realpath $(app_dir)))" \
		$(CARGO) build $(CARGOFLAGS) \
		--target boot/$(ARCH)/$(ARCH)-$(MACHINE).json \
		--manifest-path boot/$(ARCH)/Cargo.toml
	cp $(BUILD_DIR)/cargo/$(ARCH)-$(MACHINE)/$(BUILD)/boot_$(ARCH) $(@)

starina.pe: starina.elf
	$(PROGRESS) "OBJCOPY" $(@)
	$(OBJCOPY) -O binary --strip-all $< $(@)

# TODO: Can't add "-C link-args=-Map=$(@:.elf=.map)" to RUSTFLAGS because rustc considers it as
#       a change in compiler flags. Indeed it is, but it doesn't affect the output binary.
#
#       I'll file an issue on rust-lang/rust to hear  community's opinion.
$(BUILD_DIR)/%.elf: $(app_sources) Makefile
	$(PROGRESS) "CARGO" "$(@)"
	mkdir -p $(@D)
	RUSTFLAGS="$(RUSTFLAGS)" \
	CARGO_TARGET_DIR="$(BUILD_DIR)/cargo" \
		$(CARGO) build $(CARGOFLAGS) \
		--target libs/rust/starina_api/arch/$(ARCH)/$(ARCH)-user.json \
		--manifest-path $(patsubst $(BUILD_DIR)/%.elf,%,$(@))/Cargo.toml
	cp $(BUILD_DIR)/cargo/$(ARCH)-user/$(BUILD)/$(patsubst $(BUILD_DIR)/apps/%.elf,%,$(@)) $(@)
	$(OBJCOPY) --strip-all --strip-debug $(@) $(@).stripped
