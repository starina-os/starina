.section .text
.global _start

_start:
    # Print 'H'
    li a0, 'H'
    li a7, 1  # SBI_CONSOLE_PUTCHAR
    ecall
    
    # Print 'i'
    li a0, 'i'
    li a7, 1
    ecall
    
    # Print '!'
    li a0, '!'
    li a7, 1
    ecall
    
    # Print newline
    li a0, '\n'
    li a7, 1
    ecall
    
    # Exit with unimp instruction
    unimp

