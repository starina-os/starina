ARCH    ?= riscv64
MACHINE ?= qemu-virt
RELEASE ?=            # "1" to build release version
V       ?=            # "1" to enable verbose output
STARTUP ?= apps/hello
APPS    ?= apps/ping apps/pong

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

QEMU     ?= qemu-system-riscv64
CARGO    ?= cargo
PROGRESS ?= printf "  \\033[1;96m%8s\\033[0m  \\033[1;m%s\\033[0m\\n"

RUSTFLAGS += -Z macro-backtrace
CARGOFLAGS += -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem

QEMUFLAGS += -machine virt -m 256 -bios default
QEMUFLAGS += -nographic -serial mon:stdio --no-reboot
QEMUFLAGS += -d cpu_reset,unimp,guest_errors,int -D qemu.log
QEMUFLAGS += $(if $(GDB),-gdb tcp::7789 -S)

app_elfs := $(foreach app,$(APPS),build/$(app).elf)
sources += \
	$(shell find \
		boot/$(ARCH) kernel libs apps \
		-name '*.rs' -o -name '*.json' -o -name '*.ld' -o -name '*.toml' -o -name '*.S' \
	)

.DEFAULT_GOAL := default
default: ftl.elf

.PHONY: run
run: ftl.elf
	$(PROGRESS) "QEMU" "ftl.elf"
	$(QEMU) $(QEMUFLAGS) -kernel ftl.elf

.PHONY: clean
clean:
	rm -f build

.PHONY: clippy
clippy:
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) clippy --fix --allow-dirty --allow-staged $(CARGOFLAGS) --manifest-path boot/$(ARCH)/Cargo.toml

.PHONY: fmt
fmt:
	find boot kernel libs apps tools -name '*.rs' | xargs rustup run nightly rustfmt

.PHONY: fix
fix:
	cargo clippy --fix --allow-dirty --allow-staged $(CARGOFLAGS)

ftl.elf: $(sources) libs/rust/ftl_api_autogen/lib.rs Makefile $(app_elfs)
	$(PROGRESS) "CARGO" "boot/$(ARCH)"
	RUSTFLAGS="$(RUSTFLAGS)" CARGO_TARGET_DIR="build/cargo" $(CARGO) build $(CARGOFLAGS) \
		--target boot/$(ARCH)/$(ARCH)-$(MACHINE).json \
		--manifest-path boot/$(ARCH)/Cargo.toml
	cp build/cargo/$(ARCH)-$(MACHINE)/$(BUILD)/boot_$(ARCH) $(@)

build/startup.elf: build/$(STARTUP).elf
	cp $< $@

# TODO: Can't add "-C link-args=-Map=$(@:.elf=.map)" to RUSTFLAGS because rustc considers it as
#       a change in compiler flags. Indeed it is, but it doesn't affect the output binary.
#
#       I'll file an issue on rust-lang/rust to hear  community's opinion.
build/%.elf: $(sources) libs/rust/ftl_api_autogen/lib.rs Makefile
	$(PROGRESS) "CARGO" "$(@)"
	mkdir -p $(@D)
	RUSTFLAGS="$(RUSTFLAGS)" \
	CARGO_TARGET_DIR="build/cargo" \
		$(CARGO) build $(CARGOFLAGS) \
		--target libs/rust/ftl_api/arch/$(ARCH)/$(ARCH)-user.json \
		--manifest-path $(patsubst build/%.elf,%,$(@))/Cargo.toml
	cp build/cargo/$(ARCH)-user/$(BUILD)/$(patsubst build/apps/%.elf,%,$(@)) $(@)

build/ftl_idlc: $(shell find tools/idlc -name '*.rs')
	mkdir -p $(@D)
	$(PROGRESS) "CARGO" "tools/idlc"
	RUSTFLAGS="$(RUSTFLAGS)" \
	CARGO_TARGET_DIR="build/cargo" \
		$(CARGO) build \
			$(if $(RELEASE),--release,) \
			--manifest-path tools/idlc/Cargo.toml
	mv build/cargo/$(BUILD)/ftl_idlc $(@)

libs/rust/ftl_api_autogen/lib.rs: idl.json build/ftl_idlc
	mkdir -p build
	$(PROGRESS) "ILDC" "$(@)"
	./build/ftl_idlc idl.json > build/idlstub.rs.tmp
	mv build/idlstub.rs.tmp $(@)
