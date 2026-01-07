//! Persistent state model.
//!
//! This module owns the authoritative and cached data structures stored in
//! stable memory. It is platform-aware (IC principals, cycles) and
//! intentionally does NOT represent a pure domain model in the DDD sense.
//!
//! Business orchestration and policy live in `workflow`.

pub mod canister;
pub mod stable;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        ThisError,
        cdk::types::{Cycles, Principal},
        eager_static, ic_memory,
        ids::{CanisterRole, SubnetRole},
        memory::impl_storable_bounded,
        storage::StorageError,
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::storage::prelude::*;

///
/// StorageError
///

#[derive(Debug, ThisError)]
pub enum StorageError {
    #[error(transparent)]
    StableMemory(#[from] stable::StableMemoryError),
}
