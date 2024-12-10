use core::panic::PanicInfo;

#[cfg_attr(target_os = "none", panic_handler)]
#[cfg_attr(not(target_os = "none"), allow(unused))]
fn panic(info: &PanicInfo) -> ! {
    error!(
        "[{}] \x1b[1;91mPANIC: {}\x1b[0m",
        ::core::module_path!(),
        info
    );
    loop {}
}
