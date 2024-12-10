#[cfg(all(target_arch = "x86_64"))]
mod x64;

#[cfg(all(target_arch = "aarch64"))]
mod arm64;
