// RDI: *const VsyscallPage
.section .text
.balign 16
.global start
start:
    xor rbp, rbp
    lea rsp, [rip + __stack_top]

    call start_rust

    // Reachable if main returns.
    int3

.section .bss  // .bss, not .data to avoid having zeroes in the ELF file
.balign 16     // 16-bytes alignment is required for SSE registers.
__stack_bottom:
    .space 256 * 1024
__stack_top:
