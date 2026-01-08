//! Layer-neutral identifiers and boundary-safe primitives.
//!
//! This module contains:
//! - Pure identifiers (IDs, enums, newtypes)
//! - Boundary-safe wrappers used across ops, workflow, and API
//!
//! It must not contain:
//! - Business logic
//! - Policy decisions
//! - Storage-backed types

mod access;
mod canister;
mod endpoint;
mod network;
mod subnet;

pub use access::AccessMetricKind;
pub use canister::CanisterRole;
pub use endpoint::{EndpointCall, EndpointCallKind, EndpointId};
pub use network::BuildNetwork;
pub use subnet::SubnetRole;
