.text
.section ".text.boot", "ax"
.global boot
boot:
    // Set the stack pointer.
    ldr  x30, =__boot_stack_top
    mov  sp, x30

    // Initialize the user library and run main().
    bl arm64_boot

    b .
