pub trait Worker {
    fn init() -> Self where Self: Sized;
}
