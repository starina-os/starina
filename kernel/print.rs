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
        write!($crate::print::Printer, $($arg)*).ok();
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

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)+) => {{
        use $crate::print::LogLevel;

        const RESET_COLOR: &str = "\x1b[0m";
        let (color, level_str) = match $level {
            LogLevel::Error => ("\x1b[91m", "ERR"),
            LogLevel::Warn =>  ("\x1b[33m", "WARN"),
            LogLevel::Info =>  ("\x1b[96m", "INFO"),
            LogLevel::Debug => ("\x1b[0m", "DEBUG"),
            LogLevel::Trace => ("\x1b[0m", "TRACE"),
        };

        $crate::println!(
            "[kernel      ] {}{:6}{} {}",
            color,
            level_str,
            RESET_COLOR,
            format_args!($($arg)+)
        );
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Error, $($arg)+) }
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Warn, $($arg)+) }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Info, $($arg)+) }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Debug, $($arg)+) }
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Trace, $($arg)+) }
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
