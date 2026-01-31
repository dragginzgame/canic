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

mod canister;
mod endpoint;
mod intent;
mod metrics;
mod network;
mod subnet;

pub use canister::CanisterRole;
pub use endpoint::{EndpointCall, EndpointCallKind, EndpointId};
pub use intent::IntentResourceKey;
pub use metrics::{AccessMetricKind, SystemMetricKind};
pub use network::BuildNetwork;
pub use subnet::SubnetRole;
