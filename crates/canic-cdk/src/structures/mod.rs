pub mod btreemap;

pub use btreemap::BTreeMap;

pub mod memory {
    pub use ic_stable_structures::memory_manager::*;
}

pub use ic_stable_structures::*;
