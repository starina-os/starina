const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";

macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("{}error: {}{}", RED, format_args!($($arg)*), RESET);
    }
}
