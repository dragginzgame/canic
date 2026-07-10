//! Module: ops
//!
//! Responsibility: expose deterministic state access and approved single-step platform effects.
//! Does not own: endpoint authentication, workflow orchestration, or pure policy decisions.
//! Boundary: workflow calls ops after authorization and before model/storage effects.
//!
//! Ops APIs are exposed via lightweight `*Ops` structs with associated
//! functions. This is a deliberate namespacing choice to keep imports stable
//! and unambiguous in large modules where multiple `env`, `config`, or
//! `storage` modules may coexist.
//!
//! The use of `*Ops` types does **not** imply ownership of state or additional
//! abstraction; they are zero-cost namespaces over free functions.

pub mod auth;
#[cfg(feature = "blob-storage")]
pub mod blob_storage;
pub mod cascade;
#[cfg(feature = "blob-storage-billing")]
pub mod cashier;
pub mod config;
mod conversion;
pub mod cost_guard;
pub mod ic;
pub mod perf;
pub mod placement;
pub mod replay;
pub mod rpc;
pub mod runtime;
pub mod storage;
pub mod topology;

///
/// Prelude
///
/// Common ops imports for modules that need boundary and diagnostic types.
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
}

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// OpsError
///
/// Ops public APIs return `Result<_, InternalError>`.
/// Ops-scoped error enums stay internal so the public error surface remains uniform.
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
