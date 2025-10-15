//! Business-logic helpers that sit between endpoint handlers and the state
//! layer.
//!
//! The ops layer orchestrates multi-step workflows such as provisioning new
//! canisters,  running scaling/sharding policies, and
//! synchronizing topology snapshots. Endpoint macros call into these modules so
//! the public surface remains thin while policy, logging, and validation live
//! here.

pub mod canister;
pub mod context;
pub mod directory;
pub mod ext;
pub mod lifecycle;
pub mod request;
pub mod root;
pub mod signature;
pub mod sync;

/// Common imports for ops submodules and consumers.
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

use crate::{ThisError, memory::Env};

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
    ExtensionError(#[from] ext::ExtensionError),

    #[error(transparent)]
    RequestError(#[from] request::RequestError),

    #[error(transparent)]
    SignatureError(#[from] signature::SignatureError),

    #[error(transparent)]
    SyncError(#[from] sync::SyncError),
}

impl OpsError {
    /// Ensure the caller is the root canister.
    pub fn require_root() -> Result<(), Self> {
        if Env::is_root() {
            Ok(())
        } else {
            Err(Self::NotRoot)
        }
    }

    /// Ensure the caller is not the root canister.
    pub fn deny_root() -> Result<(), Self> {
        if Env::is_root() {
            Err(Self::IsRoot)
        } else {
            Ok(())
        }
    }
}
