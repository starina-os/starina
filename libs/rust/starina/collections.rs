pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use vec_deque::VecDeque;

pub mod hash_map {
    pub use hashbrown::hash_map::*;
}

pub mod hash_set {
    pub use hashbrown::hash_set::*;
}

pub mod vec_deque {
    pub use alloc::collections::vec_deque::*;
}
