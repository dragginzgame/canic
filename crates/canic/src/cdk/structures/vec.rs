use derive_more::{Deref, DerefMut};
use ic_stable_structures::{Memory, Storable, vec::Vec as WrappedVec};
use std::vec::Vec as StdVec;

///
/// Vec
/// wrapper around stable structures Vec
///

#[derive(Deref, DerefMut)]
pub struct Vec<V, M>
where
    V: Storable + Clone,
    M: Memory,
{
    data: WrappedVec<V, M>,
}

impl<V, M> Vec<V, M>
where
    V: Storable + Clone,
    M: Memory,
{
    #[must_use]
    pub fn init(memory: M) -> Self {
        Self {
            data: WrappedVec::init(memory),
        }
    }

    /// Export as an ordinary std Vec
    pub fn to_std_vec(&self) -> StdVec<V> {
        self.data.iter().collect()
    }

    /// Remove all elements by repeatedly popping.
    pub fn clear(&mut self) {
        while self.data.pop().is_some() {}
    }
}
