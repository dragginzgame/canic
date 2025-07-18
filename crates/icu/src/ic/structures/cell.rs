use crate::ic::structures::DefaultMemory;
use derive_more::{Deref, DerefMut};
use ic_stable_structures::{Storable, cell::Cell as WrappedCell};

///
/// Cell
/// a wrapper around Cell that uses the default DefaultMemory
///

#[derive(Deref, DerefMut)]
pub struct Cell<T: Storable>(WrappedCell<T, DefaultMemory>);

impl<T> Cell<T>
where
    T: Clone + Storable,
{
    // new
    pub fn new(memory: DefaultMemory, value: T) -> Self {
        let data = WrappedCell::new(memory, value);

        Self(data)
    }

    // init
    pub fn init(memory: DefaultMemory, default_value: T) -> Self {
        let data = WrappedCell::init(memory, default_value);

        Self(data)
    }

    // get
    // clones to make non-Copy structures easier to use
    pub fn get(&self) -> T {
        self.0.get().clone()
    }
}
