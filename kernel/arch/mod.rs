#[cfg(all(target_arch = "riscv64"))]
mod riscv64;

#[cfg(all(target_arch = "riscv64"))]
pub use riscv64::*;
