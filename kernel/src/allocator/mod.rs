#[cfg(feature = "bump-allocator")]
mod bump_allocator;
#[cfg(feature = "talc-allocator")]
mod talc_allocator;

#[cfg(feature = "bump-allocator")]
use bump_allocator::BumpAllocator as AllocatorImpl;
#[cfg(feature = "talc-allocator")]
use talc_allocator::TalcAllocator as AllocatorImpl;

#[cfg_attr(target_os = "none", global_allocator)]
#[cfg_attr(not(target_os = "none"), allow(unused))]
pub static GLOBAL_ALLOCATOR: AllocatorImpl = AllocatorImpl::new();
