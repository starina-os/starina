ENTRY(start)

SECTIONS {
    .text :{
        *(.text .text.*);
    }

    .rodata : ALIGN(16) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(16) {
        *(.data .data.*);
    }

    .got.plt : ALIGN(8) {
        __got = .;
        *(.got);
        __got_end = .;
    }

    .rela.dyn : ALIGN(8) {
        __rela_dyn = .;
        *(.rela.dyn);
        __rela_dyn_end = .;
    }

    .bss : ALIGN(16) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);

        // TODO: Remove this and allocate from syscall dynamically.
        __heap = .;
        . += 256 * 1024;
        __heap_end = .;

        __bss_end = .;
    }
}
