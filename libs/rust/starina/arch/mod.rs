#[cfg(all(target_arch = "aarch64"))]
mod arm64;

#[cfg(all(target_arch = "aarch64"))]
pub use arm64::*;
