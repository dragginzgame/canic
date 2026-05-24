//! Stable-structure re-exports.

pub mod memory {
    pub use ic_memory::stable_structures::memory_manager::*;
}

pub use ic_memory::stable_structures::{
    BTreeSet, Cell, DefaultMemoryImpl, FileMemory, Log, Memory, MinHeap, StableBTreeSet,
    StableCell, StableLog, StableMinHeap, StableVec, Storable, Vec, VectorMemory, btreeset, cell,
    file_mem, log, min_heap, reader, storable, vec, vec_mem, writer,
};
