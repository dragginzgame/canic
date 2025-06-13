use crate::ic::structures::DefaultMemory;
use derive_more::{Deref, DerefMut};
use ic_stable_structures::{Storable, btreeset::BTreeSet as WrappedBTreeSet};

///
/// BTreeSet
/// a wrapper around BTreeSet that uses the default VirtualMemory
///

#[derive(Deref, DerefMut)]
pub struct BTreeSet<V>
where
    V: Clone + Ord + Storable,
{
    data: WrappedBTreeSet<V, DefaultMemory>,
}

impl<V> BTreeSet<V>
where
    V: Clone + Ord + Storable,
{
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self {
            data: WrappedBTreeSet::init(memory),
        }
    }
}
