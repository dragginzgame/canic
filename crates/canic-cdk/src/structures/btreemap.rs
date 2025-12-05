pub use ic_stable_structures::btreemap::*;

use derive_more::{Deref, DerefMut};
use ic_stable_structures::{Memory, Storable, btreemap::BTreeMap as WrappedBTreeMap};

///
/// BTreeMap
/// a wrapper around BTreeMap that uses the default VirtualMemory
///

#[derive(Deref, DerefMut)]
pub struct BTreeMap<K, V, M>
where
    K: Storable + Ord + Clone,
    V: Storable + Clone,
    M: Memory,
{
    data: WrappedBTreeMap<K, V, M>,
}

impl<K, V, M> BTreeMap<K, V, M>
where
    K: Storable + Ord + Clone,
    V: Storable + Clone,
    M: Memory,
{
    #[must_use]
    pub fn init(memory: M) -> Self {
        Self {
            data: WrappedBTreeMap::init(memory),
        }
    }

    pub fn view(&self) -> impl Iterator<Item = (K, V)> + '_ {
        self.iter().map(|e| (e.key().clone(), e.value()))
    }

    /// Collect all key/value pairs into a Vec.
    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.iter().map(|e| (e.key().clone(), e.value())).collect()
    }

    /// clear
    /// the original clear() method in the ic-stable-structures library
    /// couldn't be wrapped as it took ownership, so they made a new one
    pub fn clear(&mut self) {
        self.clear_new();
    }
}
