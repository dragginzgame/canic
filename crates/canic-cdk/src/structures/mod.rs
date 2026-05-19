//! Stable-structure re-exports plus a few small Canic wrappers.

pub mod btreemap;

pub use btreemap::BTreeMap;

pub mod memory {
    pub use ic_memory::stable_structures::memory_manager::*;
}

pub use ic_memory::stable_structures::*;
