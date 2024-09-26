//! Logging utilities.
//!
//! [`std::println!`](https://doc.rust-lang.org/std/macro.println.html)
//! for FTL applications.
//!
//! # Format string syntax
//!
//! See [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) for the
//! format syntax.
//!
//! # Examples
//!
//! ```
//! use ftl_api::prelude::*; // Import all logging macros.
//!
//! info!("Hello, world!");
//!
//! let answer = 42;
//! debug!("The answer is {}", answer);
//! ```

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
        use $crate::log::LogLevel;

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
    ($($arg:tt)+) => { $crate::log!($crate::log::LogLevel::Error, $($arg)+) }
}

/// Logs with [`LogLevel::Warn`].
#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => { $crate::log!($crate::log::LogLevel::Warn, $($arg)+) }
}

/// Logs with [`LogLevel::Info`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => { $crate::log!($crate::log::LogLevel::Info, $($arg)+) }
}

/// Logs with [`LogLevel::Debug`].
#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => { $crate::log!($crate::log::LogLevel::Debug, $($arg)+) }
}

/// Logs with [`LogLevel::Trace`].
#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => { $crate::log!($crate::log::LogLevel::Trace, $($arg)+) }
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
