//! Module: replay_policy::root_capability_manifest
//!
//! Responsibility: record replay policy for root-capability command variants.
//! Does not own: capability RPC execution, root workflow dispatch, or replay storage.
//! Boundary: command manifest rows consumed by replay policy tests and workflows.

use crate::replay_policy::{
    quota::{
        DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1, VALUE_TRANSFER_QUOTA_V1,
        VALUE_TRANSFER_RESERVE_V1,
    },
    types::{
        CostClass, ReplayCommandKindLabel, ReplayCycleReservePolicyLabel,
        ReplayImplementationStatus, ReplayPolicy, ReplayQuotaPolicyLabel,
        RootCapabilityCommandReplayPolicy,
    },
};

/// Canonical replay-policy rows for internal root-capability command variants.
pub const ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST: &[RootCapabilityCommandReplayPolicy] = &[
    root_capability_response_idempotent(
        "AcknowledgePlacementReceipt",
        command_kind("root.acknowledge_placement_receipt"),
    ),
    root_capability_replay_protected(
        "AllocatePlacementChild",
        command_kind("root.allocate_placement_child"),
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "ProvisionCanister",
        command_kind("root.provision"),
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "UpgradeCanister",
        command_kind("root.upgrade.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "RecycleCanister",
        command_kind("root.recycle_canister.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    root_capability_replay_protected(
        "RequestCycles",
        command_kind("root.request_cycles.v1"),
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

const fn command_kind(label: &'static str) -> ReplayCommandKindLabel {
    ReplayCommandKindLabel::new(label)
}

const fn root_capability_response_idempotent(
    variant: &'static str,
    command_kind: ReplayCommandKindLabel,
) -> RootCapabilityCommandReplayPolicy {
    RootCapabilityCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::ResponseIdempotent { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn root_capability_replay_protected(
    variant: &'static str,
    command_kind: ReplayCommandKindLabel,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
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
