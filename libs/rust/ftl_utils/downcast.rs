use core::any::Any;

pub trait Downcastable: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Downcastable for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
