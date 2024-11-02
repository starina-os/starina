//! Frequently used types and traits.
//!
//! This module contains the most common types and traits to import. Here is an
//! idiomatic way to import the prelude:
//!
//! ```
//! use starina_api::prelude::*;
//!
//! let mut v = Vec::new();
//! v.push(1);
//! ```
//!
//! # What's in the prelude?
//!
//! Since Starina apps are `#![no_std]` crates, this prelude includes types from
//! [`alloc`](https://doc.rust-lang.org/alloc/index.html) so that experienced
//! Rust programmers can use familiar types without asking *"Where is `Vec`?"*.
//!
//! # Why should I use the prelude?
//!
//! You don't have to use the prelude. However, it's a convenient way to
//! reduce the number of your keystrokes.
pub use alloc::borrow::ToOwned;
pub use alloc::boxed::Box;
pub use alloc::string::String;
pub use alloc::string::ToString;
pub use alloc::vec;
pub use alloc::vec::Vec;

pub use crate::debug;
pub use crate::debug_warn;
pub use crate::error;
pub use crate::info;
pub use crate::trace;
pub use crate::warn;
