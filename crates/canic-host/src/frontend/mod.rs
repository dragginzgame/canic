//! Frontend environment and exact binding handoff.
//!
//! The host exports browser data; Fleet admission and external asset ownership stay separate.

pub mod model;
pub mod ops;
pub mod policy;
#[cfg(test)]
mod tests;
pub mod view;
pub mod workflow;

use thiserror::Error;

/// Typed rejection at the frontend handoff boundary.
#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("frontend derivation origin differs from the reviewed Fleet admission namespace")]
    AdmissionOrigin,

    #[error("frontend artifact exceeds the configured {0} bound")]
    Bound(&'static str),

    #[error("invalid frontend Candid: {0}")]
    Candid(String),

    #[error("frontend environment or network differs from the terminal Fleet")]
    Environment,

    #[error("frontend bundle digest or artifact identity differs from the expected handoff")]
    Integrity,

    #[error(transparent)]
    Icp(#[from] crate::icp::IcpCommandError),

    #[error(transparent)]
    Inventory(#[from] crate::fleet_ensure::CurrentFleetInventoryError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Network(#[from] crate::network::NetworkIdentityError),

    #[error("external asset canister has no exact native cycle balance in its management status")]
    NativeBalance,

    #[error(
        "external asset native cycles {available} are below the reviewed floor {required}; top up the canister's native balance before upload"
    )]
    NativeCapacity { available: u128, required: u128 },

    #[error(
        "frontend origin must be canonical HTTPS, or explicit loopback HTTP for local use: {0}"
    )]
    Origin(String),

    #[error("frontend origins must be unique and authorize the selected asset origin")]
    OriginSet,

    #[error("frontend canister Principal must identify a concrete canister")]
    Principal,

    #[error(transparent)]
    ProtocolBinding(#[from] crate::protocol_binding::ProtocolBindingError),

    #[error("frontend selection has no unique terminal role binding: {0}")]
    Role(String),

    #[error("unsupported frontend schema version")]
    Schema,

    #[error(transparent)]
    State(#[from] crate::fleet_ensure::ops::EnsureStateError),
}
