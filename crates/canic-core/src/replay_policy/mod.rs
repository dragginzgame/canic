//! Module: replay_policy
//!
//! Responsibility: expose the replay-policy manifest for Canic-owned surfaces.
//! Does not own: replay receipt execution, access control, or workflow guards.
//! Boundary: policy inventory data consumed by release checks and replay workflows.

mod endpoint_manifest;
mod quota;
mod role_command_manifest;
mod root_capability_manifest;

#[cfg(test)]
mod tests;

mod types;

pub use endpoint_manifest::{ENDPOINT_REPLAY_POLICY_MANIFEST, endpoint_replay_policy_manifest};
pub use role_command_manifest::{
    COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST, MANAGED_COMMAND_REPLAY_POLICY_MANIFEST,
    ROOT_COMMAND_REPLAY_POLICY_MANIFEST, STORE_COMMAND_REPLAY_POLICY_MANIFEST,
    coordinator_command_replay_policy_manifest, managed_command_replay_policy_manifest,
    root_command_replay_policy_manifest, store_command_replay_policy_manifest,
};
pub use root_capability_manifest::{
    ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST, root_capability_command_replay_policy_manifest,
};
pub use types::{
    CommandReplayPolicy, CostClass, EndpointKind, EndpointReplayPolicy, ReplayCommandKindLabel,
    ReplayCommandManifestLabel, ReplayCycleReservePolicyLabel, ReplayImplementationStatus,
    ReplayPolicy, ReplayQuotaPolicyLabel,
};
