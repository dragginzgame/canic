//! Business-logic helpers that sit between endpoint handlers and the state
//! layer.
//!
//! The ops layer orchestrates multi-step workflows such as provisioning new
//! canisters,  running scaling/sharding policies, and
//! synchronizing topology snapshots. Endpoint macros call into these modules so
//! the public surface remains thin while policy, logging, and validation live
//! here.

pub mod call;
pub mod config;
pub mod http;
pub mod ic;
pub mod metrics;
pub mod model;
pub mod orchestrator;
pub mod perf;
pub mod request;
pub mod root;
pub mod runtime;
pub mod service;
pub mod sync;
pub mod wasm;

// -----------------------------------------------------------------------------
// Compatibility re-exports
// -----------------------------------------------------------------------------

pub use ic::signature;
pub use ic::{icrc, timer};
pub use model::memory::CanisterInitPayload;

/// Common imports for ops submodules and consumers.
pub mod prelude {
    pub use crate::{
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            types::{Account, Int, Nat, Principal, Subaccount},
        },
        ids::CanisterRole,
        log,
        log::Level,
        ops::{OpsError, call::Call, ic::call_and_decode},
        types::Cycles,
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::{ThisError, ops::model::memory::EnvOps};

///
/// OpsError
/// Error envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum OpsError {
    /// Raised when a function requires root context, but was called from a child.
    #[error("operation must be called from the root canister")]
    NotRoot,

    /// Raised when a function must not be called from root.
    #[error("operation cannot be called from the root canister")]
    IsRoot,

    #[error(transparent)]
    ConfigOpsError(#[from] config::ConfigOpsError),

    #[error(transparent)]
    IcOpsError(#[from] ic::IcOpsError),

    #[error(transparent)]
    ModelOpsError(#[from] model::ModelOpsError),

    #[error(transparent)]
    RequestOpsError(#[from] request::RequestOpsError),

    #[error(transparent)]
    SyncOpsError(#[from] sync::SyncOpsError),

    #[error(transparent)]
    OrchestratorError(#[from] orchestrator::OrchestratorError),

    #[error(transparent)]
    ReserveOpsError(#[from] model::memory::reserve::ReserveOpsError),
}

impl OpsError {
    /// Ensure the caller is the root canister.
    pub fn require_root() -> Result<(), Self> {
        if EnvOps::is_root() {
            Ok(())
        } else {
            Err(Self::NotRoot)
        }
    }

    /// Ensure the caller is not the root canister.
    pub fn deny_root() -> Result<(), Self> {
        if EnvOps::is_root() {
            Err(Self::IsRoot)
        } else {
            Ok(())
        }
    }
}
