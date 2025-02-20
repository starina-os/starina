ENTRY(boot)

SECTIONS {
    . = 0x80200000;

    __kernel_start = .;

    .text :{
        KEEP(*(.text.boot));

        /* For some reason #[align(4)] in functions is no longer supported in
           latest Rust nightly. This is a quick workaround. */
        . = ALIGN(4);
        *(.text.switch_to_kernel);
        . = ALIGN(4);
        *(.text.idle_entry);

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

    __free_ram = .;
    . += 64 * 1024 * 1024; /* 64 MB */
    __free_ram_end = .;
}
