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
//!
//! ## Naming and structure
//!
//! Ops APIs are exposed via lightweight `*Ops` structs with associated
//! functions. This is a deliberate namespacing choice to keep imports stable
//! and unambiguous in large modules where multiple `env`, `config`, or
//! `storage` modules may coexist.
//!
//! The use of `*Ops` types does **not** imply ownership of state or additional
//! abstraction; they are zero-cost namespaces over free functions.

pub mod auth;
pub mod cascade;
pub mod config;
pub mod ic;
pub mod perf;
pub mod placement;
pub mod rpc;
pub mod runtime;
pub mod storage;
pub mod topology;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        cdk::{
            candid::CandidType,
            types::{Account, Cycles, Principal},
        },
        ids::CanisterRole,
        log,
        log::Topic,
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// OpsError
///
/// Ops public APIs return Result<_, InternalError>.
/// Ops-scoped error enums are implementation details used to preserve structure and ownership.
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

impl From<OpsError> for InternalError {
    fn from(err: OpsError) -> Self {
        Self::ops(InternalErrorOrigin::Ops, err.to_string())
    }
}
