/// The maximum ID of a handle. 0xf_ffff (1048575) is intentional and must
/// not be changed - by design, the ID is 20 bits wide so that we can use
/// the remaining bits in some cases, e.g. in for sytem call return values.
pub const HANDLE_ID_BITS: usize = 20;
pub const HANDLE_ID_MASK: i32 = (1 << HANDLE_ID_BITS) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HandleId(i32);

impl HandleId {
    pub const fn from_raw(id: i32) -> HandleId {
        debug_assert!(id >= 0 && id < (1 << HANDLE_ID_BITS));
        HandleId(id)
    }

    pub const fn as_i32(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HandleIdWithBits(i32);

impl HandleIdWithBits {
    pub const fn from_raw(id: i32) -> HandleIdWithBits {
        HandleIdWithBits(id)
    }

    pub const fn id(self) -> HandleId {
        HandleId::from_raw(self.0 & HANDLE_ID_MASK)
    }

    pub const fn bits(self) -> u8 {
        (self.0 >> HANDLE_ID_BITS) as u8
    }
}
