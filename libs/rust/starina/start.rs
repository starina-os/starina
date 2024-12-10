use crate::allocator;

pub struct Environ;

extern "Rust" {
    fn main(env: Environ);
}

#[no_mangle]
pub unsafe extern "C" fn start_rust() {

    allocator::init();

    main(Environ);
}
