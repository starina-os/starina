use core::panic::PanicInfo;

use crate::println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[{}] \x1b[1;91mPANIC: {}\x1b[0m", ::core::module_path!(), info);
    loop {}
}
