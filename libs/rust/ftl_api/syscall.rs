use ftl_types::error::FtlError;
use ftl_types::syscall::SyscallNumber;
use ftl_types::syscall::VsyscallPage;
use spin::Mutex;

static VSYSCALL_PAGE: Mutex<Option<&'static VsyscallPage>> = Mutex::new(None);

pub fn syscall(
    n: SyscallNumber,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> Result<isize, FtlError> {
    let vsyscall = VSYSCALL_PAGE.lock().expect("vsyscall not set");
    (vsyscall.entry)(n as isize, a0, a1, a2, a3, a4, a5)
}

pub fn print(s: &[u8]) -> Result<(), FtlError> {
    syscall(
        SyscallNumber::Print,
        s.as_ptr() as isize,
        s.len() as isize,
        0,
        0,
        0,
        0,
    )?;
    Ok(())
}

pub(crate) fn set_vsyscall(vsyscall: &'static VsyscallPage) {
    *VSYSCALL_PAGE.lock() = Some(vsyscall);
}
