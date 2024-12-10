//! Logging utilities.
//!
//! [`std::println!`](https://doc.rust-lang.org/std/macro.println.html)
//! for Starina applications.
//!
//! # Format string syntax
//!
//! See [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) for the
//! format syntax.
//!
//! # Examples
//!
//! ```
//! use starina_api::prelude::*; // Import all logging macros.
//!
//! info!("Hello, world!");
//!
//! let answer = 42;
//! debug!("The answer is {}", answer);
//! ```

//! Low-level printing utilities.
//!
//! # Prefer [`mod@crate::log`] over `print!`
//!
//! This module is a low-level utilities for [`mod@crate::log`] module. Use [`mod@crate::log`]
//! instead of this module.
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
                let _ = syscall::console_write(self.buffer.as_bytes());
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

/// The log level.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Logs a message.
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)+) => {{
        use $crate::print::LogLevel;

        if cfg!(debug_assertions) || $level <= LogLevel::Info {
            const RESET_COLOR: &str = "\x1b[0m";
            let (color, level_str) = match $level {
                LogLevel::Error => ("\x1b[91m", "ERR"),
                LogLevel::Warn =>  ("\x1b[33m", "WARN"),
                LogLevel::Info =>  ("\x1b[96m", "INFO"),
                LogLevel::Debug => ("\x1b[0m", "DEBUG"),
                LogLevel::Trace => ("\x1b[0m", "TRACE"),
            };

            $crate::println!(
                "[{:12}] {}{:6}{} {}",
                ::core::module_path!(),
                color,
                level_str,
                RESET_COLOR,
                format_args!($($arg)+)
            );
        }
    }};
}

/// Logs with [`LogLevel::Error`].
#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Error, $($arg)+) }
}

/// Logs with [`LogLevel::Warn`].
#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Warn, $($arg)+) }
}

/// Logs with [`LogLevel::Info`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Info, $($arg)+) }
}

/// Logs with [`LogLevel::Debug`].
#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Debug, $($arg)+) }
}

/// Logs with [`LogLevel::Trace`].
#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => { $crate::log!($crate::print::LogLevel::Trace, $($arg)+) }
}

/// Similar to [`warn!`], but only logs in debug mode.
#[macro_export]
macro_rules! debug_warn {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            $crate::warn!($($arg)+);
        }
    };
}
