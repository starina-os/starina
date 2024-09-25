use core::fmt;
use core::num::NonZeroI32;
use core::str;
use core::str::Utf8Error;

use ftl_utils::static_assert;

use crate::handle::HandleId;

#[derive(Debug, PartialEq, Eq)]
pub struct TooManyItemsError;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct BytesField<const CAP: usize> {
    len: u16,
    data: [u8; CAP],
}

impl<const CAP: usize> BytesField<CAP> {
    pub const fn new(data: [u8; CAP], len: u16) -> Self {
        Self { len, data }
    }

    pub const fn zeroed() -> Self {
        Self {
            len: 0,
            data: [0; CAP],
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    pub const fn copy_from_slice(&mut self, bytes: &[u8]) {
        assert!(bytes.len() <= CAP);
        self.len = bytes.len() as u16;

        // SAFETY: The assertion above guarantees bytes.len() is not too long.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.data.as_mut_ptr(), bytes.len());
        }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }
}

impl<const CAP: usize> TryFrom<&[u8]> for BytesField<CAP> {
    type Error = TooManyItemsError;

    fn try_from(value: &[u8]) -> Result<BytesField<CAP>, TooManyItemsError> {
        debug_assert!(CAP <= u16::MAX as usize); // FIXME: This assertion doesn't work

        if value.len() > CAP {
            return Err(TooManyItemsError);
        }

        // Zeroing the remaining part is very important to avoid leak
        // information - the kernel will keep copying the whole CAP bytes
        // regardless of the length!
        let mut data: [u8; CAP] = [0; CAP];

        data[..value.len()].copy_from_slice(value);

        Ok(BytesField::new(data, value.len() as u16))
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct StringField<const CAP: usize>(BytesField<CAP>);

impl<const CAP: usize> StringField<CAP> {
    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(self.0.as_slice())
    }
}

impl<const CAP: usize> fmt::Debug for StringField<CAP> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.to_str() {
            Ok(s) => write!(f, "{:?}", s),
            Err(_) => write!(f, "(invalid utf-8 string)"),
        }
    }
}

impl<const CAP: usize> TryFrom<&str> for StringField<CAP> {
    type Error = TooManyItemsError;

    fn try_from(value: &str) -> Result<StringField<CAP>, TooManyItemsError> {
        let bytes = BytesField::try_from(value.as_bytes())?;
        Ok(StringField(bytes))
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct MovedHandle(NonZeroI32);

impl MovedHandle {
    pub const fn new(handle_id: HandleId) -> MovedHandle {
        // TODO: Make other handle types non-zero to avoid unwrap.
        let raw_id = match NonZeroI32::new(handle_id.as_i32()) {
            Some(raw_id) => raw_id,
            None => panic!("Handle ID must be non-zero"),
        };

        MovedHandle(raw_id)
    }

    pub fn handle_id(&self) -> HandleId {
        HandleId::from_raw(self.0.get())
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct HandleField(Option<MovedHandle>);

static_assert!(size_of::<HandleField>() == size_of::<HandleId>());

impl HandleField {
    pub fn take<T: From<MovedHandle>>(&mut self) -> Option<T> {
        self.0.take().map(From::from)
    }
}

impl From<MovedHandle> for HandleField {
    fn from(moved_handle: MovedHandle) -> HandleField {
        HandleField(Some(moved_handle))
    }
}
