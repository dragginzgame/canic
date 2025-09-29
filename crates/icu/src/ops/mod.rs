pub mod canister;
pub mod delegation;
pub mod request;
pub mod reserve;
pub mod response;
pub mod root;
pub mod scaling;
pub mod sharding;
pub mod sync;

pub mod prelude {
    pub use crate::{
        Log,
        cdk::{
            api::{canister_self, msg_caller},
            call::Call,
            candid::CandidType,
        },
        interface::{InterfaceError, ic::call_and_decode},
        log,
        ops::OpsError,
        types::{CanisterType, Cycles, Int, Nat, Principal, Subaccount},
    };
    pub use serde::{Deserialize, Serialize};
}

use crate::{
    Error, ThisError,
    config::{Config, data::Canister as CanisterConfig},
    memory::state::CanisterState,
};

///
/// OpsError
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
    DelegationError(#[from] delegation::DelegationError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),

    #[error(transparent)]
    ScalingError(#[from] scaling::ScalingError),

    #[error(transparent)]
    ShardingError(#[from] sharding::ShardingError),

    #[error(transparent)]
    SyncError(#[from] sync::SyncError),
}

impl OpsError {
    /// Ensure the caller is the root canister.
    pub fn require_root() -> Result<(), Self> {
        if CanisterState::is_root() {
            Ok(())
        } else {
            Err(Self::NotRoot)
        }
    }

    /// Ensure the caller is not the root canister.
    pub fn deny_root() -> Result<(), Self> {
        if CanisterState::is_root() {
            Err(Self::IsRoot)
        } else {
            Ok(())
        }
    }
}

// cfg_current_canister
pub fn cfg_current_canister() -> Result<CanisterConfig, Error> {
    let this_ty = CanisterState::try_get_view()?.ty;
    let cfg = Config::try_get_canister(&this_ty)?;

    Ok(cfg)
}
