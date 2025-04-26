ENTRY(boot)

SECTIONS {
    . = 0x80200000;
    __kernel_start = .;

    .text :{
        KEEP(*(.text.boot));
        *(.text .text.*);
    }

    .rodata : ALIGN(16) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(16) {
        *(.data .data.*);
    }

    .bss : ALIGN(16) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(4096);
    . += 16 * 1024 * 1024; /* 16 MB */
    __boot_stack_top = .;

    . = ALIGN(4096);
    __kernel_end = .;

    /DISCARD/ : {
        *(.comment);
        *(.note.*);
        *(.debug.*);
        *(.debug_frame);
    }
}
