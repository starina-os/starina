#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Irq(u32);

impl Irq {
    pub const fn new(irq: u32) -> Self {
        Self(irq)
    }

    pub const fn as_raw(&self) -> u32 {
        self.0
    }
}
