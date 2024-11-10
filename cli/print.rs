pub const RESET: &str = "\x1b[0m";
pub const RED: &str = "\x1b[31m";

macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("{}error: {}{}", $crate::print::RED, format_args!($($arg)*), $crate::print::RESET);
    }
}
