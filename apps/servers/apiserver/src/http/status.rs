use core::num::NonZeroU16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(NonZeroU16);

impl StatusCode {
    pub const OK: StatusCode = StatusCode(unsafe { NonZeroU16::new_unchecked(200) });

    pub const fn new(code: u16) -> Option<Self> {
        match NonZeroU16::new(code) {
            Some(code) => Some(StatusCode(code)),
            None => None,
        }
    }

    pub fn as_u16(self) -> u16 {
        self.0.get()
    }
}
