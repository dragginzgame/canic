//! Module: ids
//! Responsibility: layer-neutral identifiers and boundary-safe primitives.
//! Does not own: business logic, policy decisions, or storage-backed types.
//! Boundary: exposes pure IDs, enums, and newtypes across ops, workflow, and API.

mod canister;
pub mod capability;
mod endpoint;
mod intent;
mod metrics;
mod network;
mod subnet;

pub use canister::CanisterRole;
pub use capability as cap;
pub use endpoint::{EndpointCall, EndpointCallKind, EndpointId};
pub use intent::{IntentId, IntentResourceKey};
pub use metrics::{AccessMetricKind, SystemMetricKind};
pub use network::BuildNetwork;
pub use subnet::SubnetRole;
