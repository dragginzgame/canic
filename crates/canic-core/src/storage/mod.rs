//! Persisted record and stable-memory representation.
//!
//! This module contains passive persisted and cached representations. Model
//! owns authoritative state and storage invariants; ops owns access and
//! conversion at this boundary.
//!
//! Multi-step orchestration lives in `workflow`; pure decision helpers live in
//! `domain::policy::pure`.

pub mod canister;
pub mod stable;

///
/// Prelude
///

pub mod prelude {
    pub use crate::impl_storable_bounded;
    pub use crate::{
        cdk::types::{Cycles, Principal},
        eager_static,
        ids::{CanisterRole, SubnetRole},
        storage::StorageError,
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// StorageError
///

#[derive(Debug, ThisError)]
pub enum StorageError {
    #[error(transparent)]
    StableMemory(#[from] stable::StableMemoryError),
}

impl From<StorageError> for InternalError {
    fn from(err: StorageError) -> Self {
        Self::invariant(InternalErrorOrigin::Storage, err.to_string())
    }
}
