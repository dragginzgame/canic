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
    #[error("runtime log count invariant is inconsistent")]
    LogCountInvariant,

    #[error("runtime log sequence {0} already exists")]
    LogSequenceConflict(u64),

    #[error("runtime log sequence exhausted")]
    LogSequenceExhausted,

    #[error("runtime log timestamp regressed from {previous} to {current}")]
    LogTimestampRegressed { previous: u64, current: u64 },
}

impl From<StorageError> for InternalError {
    fn from(err: StorageError) -> Self {
        Self::invariant(InternalErrorOrigin::Storage, err.to_string())
    }
}
