#[cfg(target_arch = "x86_64")]
mod x64;

#[cfg(target_arch = "x86_64")]
pub use x64::*;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(not(target_os = "none"))]
mod user;

#[cfg(not(target_os = "none"))]
pub use user::*;
