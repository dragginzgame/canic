//! Module: component_operation
//!
//! Responsibility: review and reconcile one operator-requested Component.
//! Boundary: Root owns provisioning; the host retains exact submission authority.

pub mod model;
pub mod ops;
pub mod policy;
#[cfg(test)]
mod tests;
pub mod view;
pub mod workflow;

use std::io;
use thiserror::Error;

/// Typed host failure; remote protocol errors retain their originating diagnostic.
#[derive(Debug, Error)]
pub enum ComponentOperationError {
    #[error("Component operation authority changed: {field}; preserve the existing operation")]
    Authority { field: &'static str },

    #[error("Component operation has no Ready pool asset; review Root capacity before applying")]
    Capacity,

    #[error(transparent)]
    Inventory(#[from] crate::fleet_ensure::CurrentFleetInventoryError),

    #[error(transparent)]
    Icp(#[from] crate::icp::IcpCommandError),

    #[error("invalid Component operation label: {0}")]
    InvalidLabel(String),

    #[error("Component operation record failed integrity validation")]
    Integrity,

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Component operation has no retained review")]
    Missing,

    #[error(transparent)]
    Network(#[from] crate::network::NetworkIdentityError),

    #[error("Component operation progress conflicts with its reviewed authority")]
    Progress,

    #[error(transparent)]
    Protocol(#[from] crate::CanisterProtocolError),

    #[error(transparent)]
    ProtocolBinding(#[from] crate::protocol_binding::ProtocolBindingError),

    #[error("Component operation review digest differs from --review")]
    Review,

    #[error("Component Spec is not admitted on the selected Root")]
    Spec,

    #[error(transparent)]
    State(#[from] crate::fleet_ensure::ops::EnsureStateError),
}
