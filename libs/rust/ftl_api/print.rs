use alloc::string::String;

use spin::Mutex;

use crate::syscall;

const MAX_BUFFER_SIZE: usize = 1024;

pub static GLOBAL_PRINTER: Mutex<Printer> = Mutex::new(Printer {
    buffer: String::new(),
});

pub struct Printer {
    buffer: String,
}

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            if c == '\n' || self.buffer.len() >= MAX_BUFFER_SIZE {
                let _ = syscall::print(self.buffer.as_bytes());
                self.buffer.clear();
            } else {
                self.buffer.push(c);
            }
        }

        Ok(())
    }
}

/// Prints a string without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        #![allow(unused_imports)]
        use core::fmt::Write;

        let mut printer = $crate::print::GLOBAL_PRINTER.lock();
        write!(printer, $($arg)*).ok();
    }};
}

/// Prints a string and a newline.
#[macro_export]
macro_rules! println {
    () => {{
        $crate::print!(
            "\n"
        );
    }};
    ($fmt:expr) => {{
        $crate::print!(
            concat!($fmt, "\n")
        );
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!(
            concat!($fmt, "\n"),
            $($arg)*
        );
    }};
}
