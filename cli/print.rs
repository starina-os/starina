macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("{}error: {}", format_args!($($arg)*));
    }
}
