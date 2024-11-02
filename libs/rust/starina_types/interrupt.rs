#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Irq(usize);

impl Irq {
    pub fn from_raw(value: usize) -> Irq {
        Irq(value)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}
