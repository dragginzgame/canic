//! Operations layer.
//!
//! Ops functions are fallible and must not trap.
//! All unrecoverable failures are handled at lifecycle boundaries.
//!
//! This module contains two kinds of operations:
//!
//! 1. **Control ops**
//!    - Mutate state
//!    - Perform single-step platform side effects
//!    - Call IC management APIs
//!    - Must be invoked via workflow
//!
//! 2. **View ops**
//!    - Read-only facades over internal state
//!    - Perform snapshotting, aggregation, pagination
//!    - Safe to call directly from query endpoints
//!
//! Examples of view ops include registry exports and metrics views.

pub(crate) mod adapter;
pub mod config;
pub mod ic;
pub mod icrc;
pub mod perf;
pub mod rpc;
pub mod runtime;
pub mod storage;
pub mod view;

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
            types::{Account, Cycles, Int, Nat, Principal, Subaccount},
        },
        ids::CanisterRole,
        log,
        log::Level,
        ops::OpsError,
        ops::ic::{call::Call, call_and_decode},
    };
    pub use serde::{Deserialize, Serialize};
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
    RpcOps(#[from] rpc::RpcOpsError),

    #[error(transparent)]
    RuntimeOps(#[from] runtime::RuntimeOpsError),

    #[error(transparent)]
    StorageOps(#[from] storage::StorageOpsError),
}
