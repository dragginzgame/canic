//! Module: replay_policy::root_capability_manifest
//! Responsibility: record replay policy for root-capability command variants.
//! Boundary: owns manifest data only; capability RPC execution stays elsewhere.

use crate::replay_policy::{
    quota::{
        DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1, VALUE_TRANSFER_QUOTA_V1,
        VALUE_TRANSFER_RESERVE_V1,
    },
    types::{
        CostClass, ReplayImplementationStatus, ReplayPolicy, RootCapabilityCommandReplayPolicy,
    },
};

/// Canonical replay-policy rows for `RootCapabilityCommand` variants.
pub const ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST: &[RootCapabilityCommandReplayPolicy] = &[
    root_capability_replay_protected(
        "ProvisionCanister",
        "root.provision.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "UpgradeCanister",
        "root.upgrade.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "RecycleCanister",
        "root.recycle_canister.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "RequestCycles",
        "root.request_cycles.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
];

/// Returns the canonical root-capability command replay-policy manifest.
#[must_use]
pub const fn root_capability_command_replay_policy_manifest()
-> &'static [RootCapabilityCommandReplayPolicy] {
    ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
}

const fn root_capability_replay_protected(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> RootCapabilityCommandReplayPolicy {
    RootCapabilityCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::ReplayProtected {
            command_kind,
            requires_operation_id: true,
        },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}
