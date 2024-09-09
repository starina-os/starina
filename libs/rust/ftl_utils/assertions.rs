/// Static (aka compile-time) assertion macro.
///
/// Ported from rust-lang/rust (MIT/Apache-2.0): <https://github.com/rust-lang/rust/blob/432fffa8afb8fcfe658e6548e5e8f10ad2001329/library/std/src/io/error/repr_bitpacked.rs#L352>
#[macro_export]
macro_rules! static_assert {
    ($condition:expr) => {
        const _: () = assert!(
            $condition,
            concat!(
                "\n\nSTATIC ASSERTION FAILURE: the following condition is not met:\n\n    ",
                file!(),
                ":",
                stringify!(line!()),
                "\n\n"
            )
        );
    };
}
