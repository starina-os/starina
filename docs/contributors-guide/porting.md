# Porting

Starina is designed to be highly portable. Arch-dependent code is clearly separated in the codebase, supports Device Tree, and leverages Cargo's build system to cross-compile OS.

## Architecture Overview

Starina's portability comes from:

- **Clean Architecture Separation**: Platform-specific code isolated in `kernel/src/arch/`
- **Device Tree Support**: Hardware discovery through standard device tree format
- **Rust Target System**: Cross-compilation support through Rust toolchain
- **QEMU Integration**: Standardized emulation environment for testing

## Porting to a new board

If Starina already supports the ISA (e.g. `riscv64`), it would be easy to port. You'll need to change:

- Kernel entry point. For example, `riscv64_boot` expects hart ID in `a0`, and device tree blob in `a1` register on entry. Your board's bootloader may have a different protocol when entering the kernel.
- The base address in `kernel.ld` to match the board's memory map.
- Kernel memory areas mapped in `kernel::arch::VmSpace::map_kernel_space`.

> [!TIP]
>
> Starina is still premature, and it does not have a fancy build config system like Linux's Kconfig. Try making it work first, hard-coded things everywhere, and then open a PR to discuss how we can generalize it.

## Porting to a new ISA (CPU architecture)

You'll need to implement the following files:

- Install Rust toolchain
- `kernel/src/arch/<arch>`: Arch-dependent code in the kernel. `kernel/src/arch/host` is a good skeleton.
- `run.sh`: Change `--target` parameter for `cargo build`, and `$QEMU` for `qemu`.

> [!TIP]
>
> To generate `kernel.json` template, you can use `rustc`'s `--print target-spec-json` feature ([documentation](https://doc.rust-lang.org/rustc/targets/custom.html)):
>
> ```bash
> # Look for a suitable target to use
> rustc --print target-list | grep x86_64
>
> # Print a template target JSON for x86_64-unknown-none
> rustc +nightly -Z unstable-options \
>   --target=x86_64-unknown-none --print target-spec-json
> ```

## Required Architecture Components

When porting to a new ISA, you need to implement these components:

### Boot Sequence (`boot.rs`)

```rust
// Architecture-specific boot entry point
#[no_mangle]
pub unsafe extern "C" fn arch_boot(hart_id: usize, device_tree: usize) {
    // Initialize CPU state
    // Setup initial page tables
    // Jump to common kernel main
}
```

### Memory Management (`vmspace.rs`, `hvspace.rs`)

```rust
pub struct VmSpace {
    page_table: PageTable,
    // ... architecture-specific fields
}

impl VmSpace {
    pub fn new() -> Result<Self, ErrorCode> {
        // Create new address space
    }
    
    pub fn map(&mut self, vaddr: VAddr, paddr: PAddr, len: usize, flags: PageFlags) -> Result<(), ErrorCode> {
        // Map virtual to physical memory
    }
}
```

### Context Switching (`thread.rs`)

```rust
#[repr(C)]
pub struct ThreadContext {
    // CPU registers to save/restore
    pub registers: [usize; NUM_REGISTERS],
    pub pc: usize,
    pub sp: usize,
}

pub fn switch_thread(prev: &mut ThreadContext, next: &ThreadContext) {
    // Save current thread state
    // Restore next thread state
}
```

### Interrupt Handling (`interrupt.rs`)

```rust
pub fn init_interrupts() {
    // Setup interrupt vector table
    // Enable interrupt controller
}

pub fn handle_interrupt(interrupt_id: usize) {
    // Dispatch to appropriate handler
}
```

## Device Tree Integration

### Device Tree Parsing

Starina uses device tree for hardware discovery:

```rust
// Example device tree node
virtio-net@10001000 {
    compatible = "virtio,mmio";
    reg = <0x10001000 0x1000>;
    interrupts = <1>;
};
```

### Driver Matching

Drivers declare compatibility strings:

```rust
pub const SPEC: AppSpec = AppSpec {
    name: "my_driver", 
    env: &[EnvItem {
        name: "device_tree",
        ty: EnvType::DeviceTree {
            matches: &[DeviceMatch::Compatible("my-device,v1.0")],
        },
    }],
    // ...
};
```

## Build System Configuration

### Cargo Target Configuration

Create `kernel/src/arch/<arch>/kernel.json`:

```json
{
    "llvm-target": "riscv64-unknown-unknown",
    "data-layout": "e-m:e-p:64:64-i64:64-i128:128-n64-S128",
    "arch": "riscv64",
    "target-endian": "little",
    "target-pointer-width": "64",
    "target-c-int-width": "32",
    "os": "none",
    "executables": true,
    "linker-flavor": "ld.lld",
    "linker": "rust-lld",
    "panic-strategy": "abort",
    "relocation-model": "static",
    "disable-redzone": true,
    "features": "+m,+a,+c"
}
```

### Linker Script (`kernel.ld`)

```ld
ENTRY(riscv64_boot)

MEMORY {
    /* Adjust for your platform's memory map */
    RAM : ORIGIN = 0x80200000, LENGTH = 64M
}

SECTIONS {
    .text : {
        *(.text.boot)
        *(.text)
    } > RAM
    
    .data : {
        *(.data)
    } > RAM
    
    /* ... other sections */
}
```

### Build Script Updates

Update `run.sh` for your architecture:

```bash
# Set target architecture  
ARCH="x86_64-unknown-starina"
QEMU="qemu-system-x86_64"

# Build kernel
cargo build --target kernel/src/arch/x86_64/kernel.json

# Run with QEMU
$QEMU \
    -machine q35 \
    -cpu host \
    -kernel target/$ARCH/debug/kernel \
    # ... other QEMU options
```

## Testing Strategy

### QEMU Emulation

Each architecture should have QEMU support:

```bash
# RISC-V
qemu-system-riscv64 -machine virt -cpu rv64 

# x86_64  
qemu-system-x86_64 -machine q35 -cpu host

# AArch64
qemu-system-aarch64 -machine virt -cpu cortex-a72
```

### Hardware Testing

For real hardware testing:

1. **Boot from SD card/USB**: Create bootable image
2. **Serial console**: Use UART for debugging output
3. **Network boot**: PXE/TFTP boot for development
4. **JTAG debugging**: Hardware debugger integration

## Example: ARM64 Port

Here's what an ARM64 port might look like:

### Directory Structure
```
kernel/src/arch/aarch64/
├── boot.rs          # Boot assembly and early init
├── cpuvar.rs        # CPU-local variables  
├── hvspace.rs       # Hypervisor address space
├── interrupt.rs     # ARM GIC interrupt controller
├── kernel.json      # Rust target specification
├── kernel.ld        # Linker script
├── mod.rs           # Architecture module
├── thread.rs        # Thread context switching
└── vmspace.rs       # Virtual memory management
```

### Boot Implementation

```rust
// boot.rs
#[naked]
#[no_mangle]
pub unsafe extern "C" fn aarch64_boot() -> ! {
    asm!(
        // Disable interrupts
        "msr daifset, #0xf",
        
        // Setup initial stack
        "adrp x0, __stack_top",
        "mov sp, x0",
        
        // Jump to Rust
        "bl {main}",
        
        main = sym crate::main,
        options(noreturn)
    );
}
```

### Memory Management

```rust
// vmspace.rs - ARM64 specific page table implementation
impl VmSpace {
    fn map_page_arm64(&mut self, vaddr: VAddr, paddr: PAddr, flags: PageFlags) -> Result<(), ErrorCode> {
        // ARM64 4-level page table setup
        let l0_index = (vaddr.as_usize() >> 39) & 0x1ff;
        let l1_index = (vaddr.as_usize() >> 30) & 0x1ff;
        let l2_index = (vaddr.as_usize() >> 21) & 0x1ff;
        let l3_index = (vaddr.as_usize() >> 12) & 0x1ff;
        
        // Navigate page table hierarchy
        // Set page table entry with ARM64 specific flags
    }
}
```

## Performance Considerations

### Architecture-Specific Optimizations

- **RISC-V**: Use compressed instructions (`C` extension) for code density
- **ARM64**: Leverage NEON instructions for vectorized operations  
- **x86_64**: Use SIMD instructions and efficient calling conventions

### Memory Layout Optimization

```rust
// Optimize for target architecture's cache line size
#[repr(align(64))]  // x86_64 cache line
pub struct CacheAligned<T> {
    inner: T,
}
```

## Validation and Testing

### Cross-Platform CI

```yaml
# GitHub Actions example
strategy:
  matrix:
    arch: [riscv64, aarch64, x86_64]
    
steps:
  - name: Build for ${{ matrix.arch }}
    run: |
      cargo build --target kernel/src/arch/${{ matrix.arch }}/kernel.json
      
  - name: Test with QEMU
    run: |
      ./run.sh --arch ${{ matrix.arch }} --test
```

### Architecture Test Suite

```rust
// Arch-specific tests
#[cfg(test)]
mod arch_tests {
    use super::*;
    
    #[test]
    fn test_page_table_creation() {
        let vmspace = VmSpace::new().unwrap();
        assert!(vmspace.is_valid());
    }
    
    #[test] 
    fn test_context_switch() {
        let ctx1 = ThreadContext::new();
        let ctx2 = ThreadContext::new();
        
        // Test context switching preserves state
        switch_thread(&mut ctx1, &ctx2);
        switch_thread(&mut ctx2, &ctx1);
    }
}
```

## Documentation Requirements

When submitting a port, include:

- **Architecture Overview**: CPU features, memory model, instruction set
- **Build Instructions**: Toolchain setup, dependencies, build process  
- **Testing Guide**: QEMU setup, hardware requirements, known limitations
- **Performance Notes**: Benchmarks, optimization opportunities
- **Hardware Support**: Tested boards, peripheral drivers, device tree examples

## Community Support

### Getting Help

- **Discord/Matrix**: Real-time discussion with maintainers
- **GitHub Issues**: Bug reports and feature requests
- **Documentation**: Architecture-specific porting guides

### Contributing Back

1. Start with QEMU emulation support
2. Add basic functionality (boot, memory, interrupts)
3. Submit incremental PRs for review
4. Add hardware-specific drivers
5. Document the porting process

This ensures your architecture port integrates well with Starina's design principles and can be maintained long-term.
