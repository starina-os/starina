/// The console output writer.
///
/// This is an internal implementation detail of the `print!` and `println!`
/// macros. You should use those macros, not this struct directly.
pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        crate::arch::console_write(s.as_bytes());
        Ok(())
    }
}

/// Prints a string without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        #![allow(unused_imports)]
        use core::fmt::Write;
        write!($crate::print::Printer, "{}", format_args!($($arg)*)).ok();
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

/// Print kernel message with backtraces.
#[macro_export]
macro_rules! oops {
    ($($args:tt)*) => {{
        $crate::println!($($args)*);
        let mut i = 0;
        $crate::arch::backtrace(|addr| {
            if cfg!(target_pointer_width = "64") {
                $crate::println!("  #{} at {:016x}", i, addr);
            } else {
                $crate::println!("  #{} at {:08x}", i, addr);
            }

            i += 1;
        });
    }};
}
