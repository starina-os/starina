/// The maximum ID of a handle. 0xf_ffff (1048575) is intentional and must
/// not be changed - by design, the ID is 20 bits wide so that we can use
/// the remaining bits in some cases, e.g. in for sytem call return values.
pub const HANDLE_ID_BITS: usize = 20;
pub const HANDLE_ID_MASK: i32 = (1 << HANDLE_ID_BITS) - 1;

/// A handle ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleId(i32);

impl HandleId {
    pub const fn from_raw(id: i32) -> HandleId {
        Self(id)
    }

    pub fn from_raw_isize_truncated(id: isize) -> HandleId {
        // value & HANDLE_ID_MASK allows the compiler to assume we just need the
        // the lower 32-bits.
        //
        // https://godbolt.org/z/Yjc4bfhzs
        let id_u32: u32 = ((id as usize) & (HANDLE_ID_MASK as usize))
            .try_into()
            .unwrap();
        HandleId(id_u32 as i32)
    }

    pub const fn as_isize(self) -> isize {
        self.0 as isize
    }

    pub const fn as_i32(self) -> i32 {
        self.0
    }
}

/// Allowed operations on a handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleRights(pub u8);

impl HandleRights {
    pub const NONE: HandleRights = HandleRights(0);
}
