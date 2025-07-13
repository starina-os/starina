use core::num::NonZeroU16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(NonZeroU16);

impl StatusCode {
    pub const OK: StatusCode = StatusCode::new_unchecked(200);
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode::new_unchecked(500);

    pub const fn new_unchecked(code: u16) -> Self {
        StatusCode(unsafe { NonZeroU16::new_unchecked(code) })
    }

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
