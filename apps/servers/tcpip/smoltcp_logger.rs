use starina::prelude::*;

/// A logger implementation for `log` crate so that we can observe what's
/// happening in smoltcp.
struct Logger;

static LOGGER: Logger = Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        debug!(
            "{}: {}",
            record.module_path().unwrap_or("(unknown)"),
            record.args()
        );
    }
}

// TODO: Move this log crate support to starina crate. Other libraries may use log
//       crate too.
pub fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });
}
