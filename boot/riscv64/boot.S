// The kernel entrypoint for RISC-V machines. We expect Linux's RISC-V boot
// requirements:
//
//   - a0: THe hartid of this CPU.
//   - a1: The address of the device tree blob.
.section ".text.boot", "ax"
.global boot
boot:
    // Note: Don't modify a0, a1 registers here: they are used as arguments to
    //       riscv64_boot.
    mv ra, zero
    mv fp, zero
    la sp, __boot_stack_top
    j riscv64_boot
