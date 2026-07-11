//! Module: cdk::structures
//!
//! Responsibility: re-export stable-structure types used by Canic storage code.
//! Does not own: schema definitions, memory allocation policy, or migrations.
//! Boundary: keeps external stable-structure imports behind the Canic CDK facade.

pub mod memory {
    pub use ic_stable_structures::memory_manager::*;
}

pub use ic_stable_structures::{
    BTreeMap, BTreeSet, Cell, DefaultMemoryImpl, FileMemory, Log, Memory, MinHeap, StableBTreeMap,
    StableBTreeSet, StableCell, StableLog, StableMinHeap, StableVec, Storable, Vec, VectorMemory,
    btreemap, btreeset, cell, file_mem, log, min_heap, reader, storable, vec, vec_mem, writer,
};
