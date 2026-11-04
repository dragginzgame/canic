//! Module: cdk::structures
//!
//! Responsibility: expose governed storage adapters used by Canic storage code.
//! Does not own: schema definitions or memory allocation policy.
//! Boundary: collections consume exclusive IC Memory leases through IC Storage.

pub use ic_storage::adapter::{StableBTreeMap, StableCell, StableVec, Storable, storable};
pub use ic_storage::{DefaultMemoryImpl, Memory, VectorMemory};
