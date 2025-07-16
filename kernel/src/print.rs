use crate::spinlock::SpinLock;
use crate::utils::ring_buffer::RingBuffer;

/// The console output writer.
///
/// This is an internal implementation detail of the `print!` and `println!`
/// macros. You should use those macros, not this struct directly.
pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        crate::arch::console_write(bytes);
        LOG_BUFFER.lock().write(bytes);
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

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
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

        if cfg!(debug_assertions) || $level <= LogLevel::Info {
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
        }
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
    }};
}

#[macro_export]
macro_rules! debug_warn {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            $crate::warn!($($arg)+);
        }
    };
}

const LOG_BUFFER_SIZE: usize = 16 * 1024;

pub static LOG_BUFFER: SpinLock<RingBuffer<u8, LOG_BUFFER_SIZE>> = SpinLock::new(RingBuffer::new());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer_write_and_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello world";

        buffer.write(data);

        let mut read_buf = [0u8; 20];
        let read_len = buffer.read(0, &mut read_buf);

        assert_eq!(read_len, data.len());
        assert_eq!(&read_buf[..read_len], data);
    }

    #[test]
    fn test_log_buffer_wrap_around() {
        let mut buffer: RingBuffer<u8, 100> = RingBuffer::new();
        let large_data = vec![b'x'; 150];

        buffer.write(&large_data);

        let mut read_buf = [0u8; 100];
        let read_len = buffer.read(0, &mut read_buf);

        assert_eq!(read_len, 0);
    }

    #[test]
    fn test_log_buffer_partial_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"0123456789";

        buffer.write(data);

        let mut small_buf = [0u8; 5];
        let read_len = buffer.read(0, &mut small_buf);

        assert_eq!(read_len, 5);
        assert_eq!(&small_buf, b"01234");
    }

    #[test]
    fn test_log_buffer_offset_read() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello world";

        buffer.write(data);

        let mut read_buf = [0u8; 5];
        let read_len = buffer.read(6, &mut read_buf);

        assert_eq!(read_len, 5);
        assert_eq!(&read_buf, b"world");
    }

    #[test]
    fn test_log_buffer_out_of_bounds_offset() {
        let mut buffer: RingBuffer<u8, 1024> = RingBuffer::new();
        let data = b"hello";

        buffer.write(data);

        let mut read_buf = [0u8; 10];
        let read_len = buffer.read(10, &mut read_buf);

        assert_eq!(read_len, 0);
    }
}
