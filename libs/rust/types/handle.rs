#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HandleId(i32);

impl HandleId {
    pub const fn from_raw(id: i32) -> HandleId {
        HandleId(id)
    }

    pub const fn as_i32(self) -> i32 {
        self.0
    }
}
