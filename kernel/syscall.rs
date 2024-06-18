use ftl_types::error::FtlError;
use ftl_types::syscall::SyscallNumber;
use ftl_types::syscall::VsyscallPage;

pub const VSYSCALL_PAGE: VsyscallPage = VsyscallPage {
    entry: syscall_entry,
};

pub fn syscall_entry(
    n: isize,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
) -> Result<isize, FtlError> {
    match n {
        _ if n == SyscallNumber::Print as isize => {
            let bytes = unsafe { core::slice::from_raw_parts(a0 as *const u8, a1 as usize) };
            let s = core::str::from_utf8(bytes).unwrap().trim_end();
            println!("[print] {}", s);
            Ok(0)
        }
        _ => {
            println!(
                "unknown syscall: n={}, a0={}, a1={}, a2={}, a3={}, a4={}",
                n, a0, a1, a2, a3, a4
            );

            Err(FtlError::UnknownSyscall)
        }
    }
}
