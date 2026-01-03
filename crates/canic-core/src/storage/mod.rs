//! Persistent state model.
//!
//! This module owns the authoritative and cached data structures stored in
//! stable memory. It is platform-aware (IC principals, cycles) and
//! intentionally does NOT represent a pure domain model in the DDD sense.
//!
//! Business orchestration and policy live in `workflow`.

pub mod canister;
pub mod stable;

use crate::storage::stable::MemoryError;
use thiserror::Error as ThisError;

///
/// StorageError
///

#[derive(Debug, ThisError)]
pub enum StorageError {
    #[error(transparent)]
    Memory(#[from] MemoryError),
}
