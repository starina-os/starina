use crate::arch::riscv64::sbi;

pub fn console_write(bytes: &[u8]) {
    for byte in bytes {
        sbi::console_putchar(*byte);
    }
}
