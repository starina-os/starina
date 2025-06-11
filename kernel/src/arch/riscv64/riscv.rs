//! RISC-V constants and utilities.

const fn scause(intr: bool, code: u64) -> u64 {
    if intr { code | (1 << 63) } else { code }
}

pub const SCAUSE_HOST_TIMER_INTR: u64 = scause(true, 5);
pub const SCAUSE_ECALL_FROM_VS: u64 = scause(false, 10);
pub const SCAUSE_GUEST_INST_PAGE_FAULT: u64 = scause(false, 20);
pub const SCAUSE_GUEST_LOAD_PAGE_FAULT: u64 = scause(false, 21);
pub const SCAUSE_VIRTUAL_INST: u64 = scause(false, 22);
pub const SCAUSE_GUEST_STORE_PAGE_FAULT: u64 = scause(false, 23);
pub const SCAUSE_SV_EXT_INTR: u64 = scause(true, 9);

pub const OP_LOAD_FUNCT3_LB: u8 = 0;
pub const OP_LOAD_FUNCT3_LH: u8 = 1;
pub const OP_LOAD_FUNCT3_LW: u8 = 2;
pub const OP_LOAD_FUNCT3_LD: u8 = 3;

pub const OP_STORE_FUNCT3_SB: u8 = 0;
pub const OP_STORE_FUNCT3_SH: u8 = 1;
pub const OP_STORE_FUNCT3_SW: u8 = 2;
pub const OP_STORE_FUNCT3_SD: u8 = 3;
