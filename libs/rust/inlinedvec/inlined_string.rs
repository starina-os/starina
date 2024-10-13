use core::fmt;

use crate::InlinedVec;
use crate::TooManyItemsError;

/// An inlined string. Unline `String`, this type is allocated on the stack
/// or inlined in a struct instead of the heap.
///
/// The internal buffer is a `InlinedVec<u8, CAP>`, where `CAP` is the capacity
/// of the string. It's guaranteed to be a valid UTF-8 string.
pub struct InlinedString<const CAP: usize>(InlinedVec<u8, CAP>);

impl<const CAP: usize> InlinedString<CAP> {
    pub const fn new() -> Self {
        Self(InlinedVec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: We guarantee that the string is always a valid UTF-8 string.
        unsafe { core::str::from_utf8_unchecked(self.0.as_slice()) }
    }

    pub fn try_push_u8(&mut self, c: u8) -> Result<(), TooManyItemsError> {
        self.0.try_push(c).map_err(|_| TooManyItemsError)
    }
}

impl<const CAP: usize> fmt::Debug for InlinedString<CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl<const CAP: usize> fmt::Display for InlinedString<CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<const CAP: usize> TryFrom<&str> for InlinedString<CAP> {
    type Error = TooManyItemsError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut string = Self::new();
        string.0.try_extend_from_slice(s.as_bytes())?;
        Ok(string)
    }
}
