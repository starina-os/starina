/// Aligns a value down to the nearest multiple of `align`.
///
/// `align` must be a power of two.
///
/// # Example
///
/// ```
/// use ftl_utils::alignment::align_down;
///
/// assert_eq!(align_down(0x0000, 0x1000), 0x0000);
/// assert_eq!(align_down(0x0001, 0x1000), 0x0000);
/// assert_eq!(align_down(0x1000, 0x1000), 0x1000);
/// assert_eq!(align_down(0x1001, 0x1000), 0x1000);
/// assert_eq!(align_down(0x2000, 0x1000), 0x2000);
/// ```
pub const fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());

    (value) & !(align - 1)
}

/// Aligns a value up to the nearest multiple of `align`.
///
/// `align` must be a power of two.
///
/// # Example
///
/// ```
/// use ftl_utils::alignment::align_up;
///
/// assert_eq!(align_up(0x0000, 0x1000), 0x0000);
/// assert_eq!(align_up(0x0001, 0x1000), 0x1000);
/// assert_eq!(align_up(0x1000, 0x1000), 0x1000);
/// assert_eq!(align_up(0x1001, 0x1000), 0x2000);
/// assert_eq!(align_up(0x2000, 0x1000), 0x2000);
/// ```
pub const fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());

    align_down(value + align - 1, align)
}

/// Returns `true` if `value` is aligned to `align`.
///
/// `align` must be a power of two.
///
/// # Example
///
/// ```
/// use ftl_utils::alignment::is_aligned;
///
/// assert_eq!(is_aligned(0x0000, 0x1000), true);
/// assert_eq!(is_aligned(0x0001, 0x1000), false);
/// assert_eq!(is_aligned(0x1000, 0x1000), true);
/// assert_eq!(is_aligned(0x1001, 0x1000), false);
/// assert_eq!(is_aligned(0x2000, 0x1000), true);
/// ```
pub const fn is_aligned(value: usize, align: usize) -> bool {
    debug_assert!(align.is_power_of_two());

    value & (align - 1) == 0
}
