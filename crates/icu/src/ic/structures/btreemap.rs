use derive_more::{Deref, DerefMut};
use ic_stable_structures::{Memory, Storable, btreemap::BTreeMap as WrappedBTreeMap};
use std::ops::RangeBounds;

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

    /// Returns an iterator over all cloned `(K, V)` pairs.
    pub fn iter_pairs(&self) -> impl Iterator<Item = (K, V)> + '_ {
        self.iter()
            .map(|entry| (entry.key().clone(), entry.value()))
    }

    /// Returns an iterator over a range of cloned `(K, V)` pairs.
    pub fn range_pairs<R>(&self, range: R) -> impl Iterator<Item = (K, V)> + '_
    where
        R: RangeBounds<K>,
    {
        self.range(range)
            .map(|entry| (entry.key().clone(), entry.value()))
    }

    /// clear
    /// the original clear() method in the ic-stable-structures library
    /// couldn't be wrapped as it took ownership, so they made a new one
    pub fn clear(&mut self) {
        self.clear_new();
    }
}
