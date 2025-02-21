use std::io::Write;

pub fn halt() -> ! {
    panic!("halted");
}

pub fn console_write(s: &[u8]) {
    std::io::stdout().write_all(s).unwrap();
}
