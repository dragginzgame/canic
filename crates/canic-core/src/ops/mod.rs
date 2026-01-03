//! Operations layer.
//!
//! Ops functions are fallible and must not trap.
//! All unrecoverable failures are handled at lifecycle boundaries.
//!
//! This module contains operational primitives and snapshots:
//! - Mutate state and perform single-step platform side effects
//! - Read and export internal state as snapshots
//!
//! Ops must not construct DTO views or perform pagination.
//! Projection and paging are owned by workflow/query.

pub mod config;
pub mod ic;
pub mod perf;
pub mod rpc;
pub mod runtime;
pub mod storage;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        cdk::{
            candid::CandidType,
            types::{Cycles, Principal},
        },
        ids::CanisterRole,
        log,
        log::Topic,
    };
}

use crate::ThisError;

///
/// OpsError
/// Error envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum OpsError {
    #[error(transparent)]
    ConfigOps(#[from] config::ConfigOpsError),

    #[error(transparent)]
    IcOps(#[from] ic::IcOpsError),

    #[error(transparent)]
    RpcOps(#[from] rpc::RpcOpsError),

    #[error(transparent)]
    RuntimeOps(#[from] runtime::RuntimeOpsError),

    #[error(transparent)]
    StorageOps(#[from] storage::StorageOpsError),
}
