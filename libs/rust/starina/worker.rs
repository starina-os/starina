pub trait App: Send + Sync {
    fn init() -> Self where Self: Sized;
}
