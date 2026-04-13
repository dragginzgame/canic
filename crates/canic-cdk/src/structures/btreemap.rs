pub use ic_stable_structures::btreemap::*;

use ic_stable_structures::{Memory, Storable, btreemap::BTreeMap as WrappedBTreeMap};
use std::ops::{Deref, DerefMut};

///
/// BTreeMap
/// a wrapper around BTreeMap that uses the default VirtualMemory
///

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

impl<K, V, M> Deref for BTreeMap<K, V, M>
where
    K: Storable + Ord + Clone,
    V: Storable + Clone,
    M: Memory,
{
    type Target = WrappedBTreeMap<K, V, M>;

    // Expose the wrapped stable map through the wrapper transparently.
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<K, V, M> DerefMut for BTreeMap<K, V, M>
where
    K: Storable + Ord + Clone,
    V: Storable + Clone,
    M: Memory,
{
    // Expose mutable access to the wrapped stable map.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
