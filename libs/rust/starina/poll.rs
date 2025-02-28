#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Readiness(i8);

impl Readiness {
    pub const CLOSED: Readiness = Readiness(1 << 0);
    pub const READABLE: Readiness = Readiness(1 << 1);
    pub const WRITABLE: Readiness = Readiness(1 << 2);

    pub fn contains(&self, other: Readiness) -> bool {
        self.0 & other.0 != 0
    }
}
