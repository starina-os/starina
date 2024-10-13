.set INTERRUPT_HANDLER_SIZE, 16
.align INTERRUPT_HANDLER_SIZE
.global interrupt_handlers
interrupt_handlers:
.set i, 0
.rept 256
.set handler_start, .
// Exceptions with error code.
.if i == 8 || 10 <= i && i <= 14 || i == 17
    .align INTERRUPT_HANDLER_SIZE
    cli
    push i
    jmp interrupt_common
    .align INTERRUPT_HANDLER_SIZE
// Interrupts and exceptions without error code.
.else
    .align INTERRUPT_HANDLER_SIZE
    cli
    push 0 // Dummy value as error code.
    push i
    jmp interrupt_common
    .align INTERRUPT_HANDLER_SIZE
.endif

// Increment the counter.
.set i, i + 1
.endr

.extern x64_handle_interrupt
interrupt_common:
    //
    //  The current stack frame:
    //
    //            +--------------------+
    //     48     |        SS          |
    //            +--------------------+
    //     40     |        RSP         |
    //            +--------------------+
    //     32     |       RFLAGS       |
    //            +--------------------+
    //     24     |        CS          |
    //            +--------------------+
    //     16     |        RIP         |
    //            +--------------------+
    //      8     |     Error code     |
    //            +--------------------+
    //      0     |     IRQ Number     | <- RSP
    //            +--------------------+
    //

    // Save RDI and set the IRQ number to RDI at once.
    xchg rdi, [rsp]

    // Save registers except RDI (we have already saved it above).
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rbp
    push rsi
    push rdx
    push rcx
    push rbx
    push rax

    mov rsi, rsp
    call x64_handle_interrupt

    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rbp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    pop rdi

    // Skip error code.
    add rsp, 8
1:
    iretq
