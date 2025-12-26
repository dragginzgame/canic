//! Business-logic helpers that sit between endpoint handlers and the state
//! layer.
//!
//! The ops layer orchestrates multi-step workflows such as provisioning new
//! canisters,  running scaling/sharding policies, and
//! synchronizing topology snapshots. Endpoint macros call into these modules so
//! the public surface remains thin while policy, logging, and validation live
//! here.

pub mod config;
pub mod cycles;
pub mod directory;
pub mod env;
pub mod ic;
pub mod icrc;
pub mod memory;
pub mod perf;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod sharding;
pub mod state;
pub mod topology;
pub mod wasm;

use std::time::Duration;

///
/// Constants
///

/// Shared initial delay for ops timers to allow init work to settle.
pub const OPS_INIT_DELAY: Duration = Duration::from_secs(10);

/// Shared cadence for cycle tracking (10 minutes).
pub const OPS_CYCLE_TRACK_INTERVAL: Duration = Duration::from_secs(60 * 10);

/// Shared cadence for log retention (10 minutes).
pub const OPS_LOG_RETENTION_INTERVAL: Duration = Duration::from_secs(60 * 10);

/// Pool timer initial delay (30 seconds) before first check.
pub const OPS_POOL_INIT_DELAY: Duration = Duration::from_secs(30);

/// Pool check cadence (30 minutes).
pub const OPS_POOL_CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);

///
/// Prelude
///

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
        ops::{
            OpsError,
            ic::{call::Call, call_and_decode},
        },
        types::Cycles,
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::{ThisError, ops::env::EnvOps};

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
    EnvOpsError(#[from] env::EnvOpsError),

    #[error(transparent)]
    IcOpsError(#[from] ic::IcOpsError),

    #[error(transparent)]
    MemoryRegistryOpsError(#[from] memory::MemoryRegistryOpsError),

    #[error(transparent)]
    RpcOpsError(#[from] rpc::RpcOpsError),

    #[error(transparent)]
    TopologyOpsError(#[from] topology::TopologyOpsError),
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
