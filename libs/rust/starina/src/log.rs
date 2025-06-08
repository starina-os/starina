use alloc::vec::Vec;
use core::fmt;

struct Writer {
    buf: spin::Mutex<Vec<u8>>,
}

static LOGGER: Writer = Writer::new();

impl Writer {
    const fn new() -> Writer {
        Writer {
            buf: spin::Mutex::new(Vec::new()),
        }
    }

    fn write_str(&self, s: &str) {
        let mut buf = self.buf.lock();
        for b in s.bytes() {
            buf.push(b);
            if b == b'\n' {
                let old_buf = core::mem::replace(&mut *buf, Vec::with_capacity(128));
                // Do not hold the lock while writing to the console. This
                // logger could be shared between multiple apps/threads,
                // and the kernel may switch to another app/thread.
                drop(buf);
                crate::syscall::console_write(&old_buf);
                buf = self.buf.lock();
            }
        }
    }
}

/// A `Writer` wrapper. This is necessary because fmt::Write expects
/// `write_str` to be `&mut self`, but `Writer::write_str` should be
/// `&self` to be aware of its lock.
struct WriterWrapper;

impl fmt::Write for WriterWrapper {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        LOGGER.write_str(s);
        Ok(())
    }
}

impl log::Log for Writer {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level_str = match record.level() {
                log::Level::Error => "ERROR",
                log::Level::Warn => "WARN",
                log::Level::Info => "INFO",
                log::Level::Debug => "DEBUG",
                log::Level::Trace => "TRACE",
            };

            let color = match record.level() {
                log::Level::Error => "\x1b[91m",
                log::Level::Warn => "\x1b[33m",
                log::Level::Info => "\x1b[96m",
                log::Level::Debug | log::Level::Trace => "\x1b[0m",
            };

            const RESET_COLOR: &str = "\x1b[0m";

            use core::fmt::Write;
            write!(
                WriterWrapper,
                "[{:<12}] {}{:6}{} {}\n",
                crate::tls::thread_local().name,
                color,
                level_str,
                RESET_COLOR,
                record.args()
            )
            .unwrap();
        }
    }

    fn flush(&self) {
        // Console writing is immediate, no buffering to flush
    }
}

/// Initialize the logger. This should be called once at the start of the program.
pub fn init() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(if cfg!(debug_assertions) {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        });
    }
}

pub use log::debug;
pub use log::error;
pub use log::info;
pub use log::trace;
pub use log::warn;

#[macro_export]
macro_rules! debug_warn {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            $crate::log::warn!($($arg)+);
        }
    };
}
