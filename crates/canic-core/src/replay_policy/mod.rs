//! Module: replay_policy
//!
//! Responsibility: expose the replay-policy manifest for Canic-owned surfaces.
//! Does not own: replay receipt execution, access control, or workflow guards.
//! Boundary: policy inventory data consumed by release checks and replay workflows.

mod endpoint_manifest;
mod pool_admin_manifest;
mod quota;
mod root_capability_manifest;

#[cfg(test)]
mod tests;

mod types;

pub use endpoint_manifest::{ENDPOINT_REPLAY_POLICY_MANIFEST, endpoint_replay_policy_manifest};
pub use pool_admin_manifest::{
    POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST, pool_admin_command_replay_policy_manifest,
};
pub use root_capability_manifest::{
    ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST, root_capability_command_replay_policy_manifest,
};
pub use types::{
    CostClass, EndpointKind, EndpointReplayPolicy, PoolAdminCommandReplayPolicy,
    ReplayCommandKindLabel, ReplayImplementationStatus, ReplayPolicy,
    RootCapabilityCommandReplayPolicy,
};
