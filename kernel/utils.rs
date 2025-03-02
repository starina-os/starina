use core::ops::Deref;
use core::ops::DerefMut;

use hashbrown::HashMap;
use rustc_hash::FxBuildHasher;

pub struct FxHashMap<K, V>(HashMap<K, V, FxBuildHasher>);

impl<K, V> FxHashMap<K, V> {
    pub const fn new() -> Self {
        let inner = HashMap::with_hasher(FxBuildHasher);
        Self(inner)
    }
}

impl<K, V> Deref for FxHashMap<K, V> {
    type Target = HashMap<K, V, FxBuildHasher>;

    fn deref(&self) -> &HashMap<K, V, FxBuildHasher> {
        &self.0
    }
}

impl<K, V> DerefMut for FxHashMap<K, V> {
    fn deref_mut(&mut self) -> &mut HashMap<K, V, FxBuildHasher> {
        &mut self.0
    }
}
