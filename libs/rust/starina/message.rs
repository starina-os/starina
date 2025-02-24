pub const MESSAGE_NUM_HANDLES_MAX: usize = 3;

/// The message metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct MessageInfo(isize);
