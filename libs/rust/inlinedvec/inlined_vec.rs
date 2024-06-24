use core::fmt;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::RangeBounds;

#[derive(Debug, PartialEq, Eq)]
pub struct TooManyItemsError;

#[derive(Debug, PartialEq, Eq)]
pub struct CapacityError<T>(T);

pub struct InlinedVec<T, const CAP: usize>(arrayvec::ArrayVec<T, CAP>);

impl<T, const CAP: usize> InlinedVec<T, CAP> {
    pub const fn new() -> Self {
        Self(arrayvec::ArrayVec::new_const())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    pub fn try_push(&mut self, value: T) -> Result<(), CapacityError<T>> {
        self.0
            .try_push(value)
            .map_err(|err| CapacityError(err.element()))
    }

    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), TooManyItemsError>
    where
        T: Copy,
    {
        self.0
            .try_extend_from_slice(other)
            .map_err(|_| TooManyItemsError)
    }

    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }

    pub fn drain<R>(&mut self, range: R) -> arrayvec::Drain<T, CAP>
    where
        R: RangeBounds<usize>,
    {
        self.0.drain(range)
    }
}

impl<T, const CAP: usize> Deref for InlinedVec<T, CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl<T, const CAP: usize> DerefMut for InlinedVec<T, CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice()
    }
}

impl<T, const CAP: usize> AsRef<[T]> for InlinedVec<T, CAP> {
    fn as_ref(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<T, const CAP: usize> AsMut<[T]> for InlinedVec<T, CAP> {
    fn as_mut(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }
}

impl<T, const CAP: usize> fmt::Debug for InlinedVec<T, CAP>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T, const CAP: usize> Default for InlinedVec<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}
