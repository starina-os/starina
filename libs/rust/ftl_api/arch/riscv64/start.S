// a0: *const VsyscallPage
.section .text
.align 4
.global start
start:
    mv ra, zero
    mv fp, zero
    la sp, __stack_top

    call start_rust

    // Reachable if main returns.
    unimp

.section .bss  // .bss, not .data to avoid having zeroes in the ELF file
.align 4       // 16-bytes (2^4) alignment as required by RISC-V ABI
__stack_bottom:
    .space 256 * 1024
__stack_top:
