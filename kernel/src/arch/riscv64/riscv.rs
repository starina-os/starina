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
