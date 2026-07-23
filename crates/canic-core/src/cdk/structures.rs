//! Module: cdk::structures
//!
//! Responsibility: re-export stable-structure types used by Canic storage code.
//! Does not own: schema definitions, memory allocation policy, or migrations.
//! Boundary: keeps external stable-structure imports inside Canic's runtime substrate.

pub mod memory {
    pub use ic_stable_structures::memory_manager::*;
}

pub use ic_stable_structures::{
    BTreeMap, DefaultMemoryImpl, Memory, StableVec, Storable, Vec, VectorMemory, btreemap, cell,
    storable,
};
