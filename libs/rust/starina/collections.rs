pub use hash_map::HashMap;
pub use hash_set::HashSet;

pub mod hash_map {
    pub use hashbrown::hash_map::*;
}

pub mod hash_set {
    pub use hashbrown::hash_set::*;
}
