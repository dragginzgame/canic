pub mod btreemap;
pub mod cell;

pub use btreemap::BTreeMap;
pub use cell::Cell;

pub mod memory {
    pub use ic_stable_structures::memory_manager::*;
}

pub use ic_stable_structures::*;

// helper
pub type DefaultMemory =
    ic_stable_structures::memory_manager::VirtualMemory<ic_stable_structures::DefaultMemoryImpl>;
