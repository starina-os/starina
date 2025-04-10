use crate::arch::riscv64::sbi;

pub const NUM_CPUS_MAX: usize = 4;

pub fn console_write(bytes: &[u8]) {
    for byte in bytes {
        sbi::console_putchar(*byte);
    }
}

pub fn machine_init() {}
