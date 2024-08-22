// The interrupt/exception/system call handler entry point. `stvec` is set to
// this address.
//
// This function address must be aligned to 4 bytes not to accidentally set
// MODE field in stvec.
.balign 4
.global switch_to_kernel
switch_to_kernel:
    addi sp, sp, -8 * 32
    sd ra,  8 * 2(sp)
    sd gp,  8 * 3(sp)
    sd tp,  8 * 4(sp)
    sd t0,  8 * 5(sp)
    sd t1,  8 * 6(sp)
    sd t2,  8 * 7(sp)
    sd t3,  8 * 8(sp)
    sd t4,  8 * 9(sp)
    sd t5,  8 * 10(sp)
    sd t6,  8 * 11(sp)
    sd a0,  8 * 12(sp)
    sd a1,  8 * 13(sp)
    sd a2,  8 * 14(sp)
    sd a3,  8 * 15(sp)
    sd a4,  8 * 16(sp)
    sd a5,  8 * 17(sp)
    sd a6,  8 * 18(sp)
    sd a7,  8 * 19(sp)
    sd s0,  8 * 20(sp)
    sd s1,  8 * 21(sp)
    sd s2,  8 * 22(sp)
    sd s3,  8 * 23(sp)
    sd s4,  8 * 24(sp)
    sd s5,  8 * 25(sp)
    sd s6,  8 * 26(sp)
    sd s7,  8 * 27(sp)
    sd s8,  8 * 28(sp)
    sd s9,  8 * 29(sp)
    sd s10, 8 * 30(sp)
    sd s11, 8 * 31(sp)

    csrr a0, sepc
    sd a0, 8 * 0(sp)
    csrr a0, sstatus
    sd a0, 8 * 1(sp)

    mv a0, sp // *mut Frame
    call interrupt_handler

    ld a0, 8 * 0(sp)
    csrw sepc, a0
    ld a0, 8 * 1(sp)
    csrw sstatus, a0

    ld ra,  8 * 2(sp)
    ld gp,  8 * 3(sp)
    ld tp,  8 * 4(sp)
    ld t0,  8 * 5(sp)
    ld t1,  8 * 6(sp)
    ld t2,  8 * 7(sp)
    ld t3,  8 * 8(sp)
    ld t4,  8 * 9(sp)
    ld t5,  8 * 10(sp)
    ld t6,  8 * 11(sp)
    ld a0,  8 * 12(sp)
    ld a1,  8 * 13(sp)
    ld a2,  8 * 14(sp)
    ld a3,  8 * 15(sp)
    ld a4,  8 * 16(sp)
    ld a5,  8 * 17(sp)
    ld a6,  8 * 18(sp)
    ld a7,  8 * 19(sp)
    ld s0,  8 * 20(sp)
    ld s1,  8 * 21(sp)
    ld s2,  8 * 22(sp)
    ld s3,  8 * 23(sp)
    ld s4,  8 * 24(sp)
    ld s5,  8 * 25(sp)
    ld s6,  8 * 26(sp)
    ld s7,  8 * 27(sp)
    ld s8,  8 * 28(sp)
    ld s9,  8 * 29(sp)
    ld s10, 8 * 30(sp)
    ld s11, 8 * 31(sp)
    addi sp, sp, 8 * 32
    sret
