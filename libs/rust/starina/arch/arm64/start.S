// x0: *const VsyscallPage
.text
.global start
start:
    // Set the stack pointer.
    adr  x30, __stack_top
    mov  sp, x30

    // Initialize the user library and run main().
    bl start_rust

    // Unreachable here.
    brk 0

.section .bss  // .bss, not .data to avoid having zeroes in the ELF file
.balign 16
__stack_bottom:
    .space 256 * 1024
__stack_top:
