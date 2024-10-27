.section ".text.boot", "ax"

// https://www.kernel.org/doc/Documentation/arm64/booting.txt
linux_image_header:
    nop              // code0
    b boot           // code1
    .quad 0          // text_offset
    .quad 0          // image_size
    .quad 0          // flags
    .quad 0          // res2
    .quad 0          // res3
    .quad 0          // res4
    .ascii "ARM\x64" // magic
    .long 0          // res5 (PE/COFF offset)

.text
.global boot
boot:

// BUild a page table, identity mapped.
build_page_table:
    // Level-0 table: Only 1 entry, pointing to the L1 table.
    adrp x10, __page_table_l0
    add  x10, x10, :lo12:__page_table_l0
    adrp x11, __page_table_l1
    add  x11, x11, :lo12:__page_table_l1
    mov  x12, #0x00000403 // nG (not global), AttrIndex = 0, Table
    orr  x11, x11, x12
    str  x11, [x10], #8

    // Level-1 table: 3 entries, pointing to the L2 table.
    adrp x10, __page_table_l1
    add  x10, x10, :lo12:__page_table_l1
    adrp x11, __page_table_l2_0
    add  x11, x11, :lo12:__page_table_l2_0
    mov  x12, #0x00000b03 // nG (not global), AttrIndex = 0, Table
    orr  x11, x11, x12
    str  x11, [x10], #8

    adrp x11, __page_table_l2_1
    add  x11, x11, :lo12:__page_table_l2_1
    mov  x12, #0x00000b03 // nG (not global), AttrIndex = 0, Table
    orr  x11, x11, x12
    str  x11, [x10], #8

    adrp x11, __page_table_l2_2
    add  x11, x11, :lo12:__page_table_l2_2
    mov  x12, #0x00000b03 // nG (not global), AttrIndex = 0, Table
    orr  x11, x11, x12
    str  x11, [x10], #8

    // Level-2 table: 0x00000000-0x3fffffff (1GB)
    adrp x10, __page_table_l2_0
    add  x10, x10, :lo12:__page_table_l2_0
    mov x11, #0x00000000 // paddr base
    mov x12, #0x00000401 // nG (not global), AttrIndex = 0, Block
    orr x11, x11, x12
    mov x12, #512        // 512 entries
    mov x13, #0x200000   // 2MB block size
1:
    str x11, [x10], #8
    add x11, x11, x13
    subs x12, x12, #1
    bne 1b

    // Level-2 table: 0x40000000-0x7fffffff (1GB)
    adrp x10, __page_table_l2_1
    add  x10, x10, :lo12:__page_table_l2_1
    mov x11, #0x40000000 // paddr base
    mov x12, #0x00000401 // nG (not global), AttrIndex = 0, Block
    orr x11, x11, x12
    mov x12, #512        // 512 entries
    mov x13, #0x200000   // 2MB block size
2:
    str x11, [x10], #8
    add x11, x11, x13
    subs x12, x12, #1
    bne 2b

    // Level-2 table: 0x80000000-0xbfffffff (1GB)
    adrp x10, __page_table_l2_2
    add  x10, x10, :lo12:__page_table_l2_2
    mov x11, #0x80000000 // paddr base
    mov x12, #0x00000401 // nG (not global), AttrIndex = 0, Block
    orr x11, x11, x12
    mov x12, #512        // 512 entries
    mov x13, #0x200000   // 2MB block size
3:
    str x11, [x10], #8
    add x11, x11, x13
    subs x12, x12, #1
    bne 3b

enable_mmu:
    // Configure paging.
    ldr  x10, =0x000000ff // AttrIndx == 0: Normal memory, cache enabled
                          // AttrIndx == 1: Device-nGnRnE memory
    msr  mair_el1, x10

    ldr  x10, =0x35103510 // 4KiB pages, the region size is 2^48.
    msr  tcr_el1, x10

    adrp x10, __page_table_l0
    add  x10, x10, :lo12:__page_table_l0
    msr  ttbr1_el1, x10
    msr  ttbr0_el1, x10

    ldr  x10, =0x30D01805 // Lttile Endian, MMU enabled, enable I/D-cache
    dsb ish
    isb
    msr  sctlr_el1, x10
    isb

jump_to_rust:
    // Set the stack pointer.
    adrp   x30, __boot_stack_top
    add    x30, x30, :lo12:__boot_stack_top
    mov sp, x30

    bl arm64_boot
    b .

.section ".data"
.balign 4096
__page_table_l0:
    .space 4096

.balign 4096
__page_table_l1:
    .space 4096

.balign 4096
__page_table_l2_0:
    .space 4096

.balign 4096
__page_table_l2_1:
    .space 4096

.balign 4096
__page_table_l2_2:
    .space 4096

.rodata
.balign 4096
.global __vsyscall_page
__vsyscall_page:
    svc 0
    ret
