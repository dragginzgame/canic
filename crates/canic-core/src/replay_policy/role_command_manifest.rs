//! Module: replay_policy::role_command_manifest
//!
//! Responsibility: record replay policy for each role-owned command variant.
//! Does not own: role dispatch, authorization, workflow execution, or replay storage.
//! Boundary: the common method dispatches through one of these role-owned manifests.

use crate::replay_policy::{
    quota::{
        DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1, DURABLE_PUBLISH_QUOTA_V1,
        DURABLE_PUBLISH_RESERVE_V1, ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1,
        ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1, ROOT_CHAIN_KEY_SIGNING_QUOTA_V1,
        VALUE_TRANSFER_QUOTA_V1, VALUE_TRANSFER_RESERVE_V1,
    },
    types::{
        CommandReplayPolicy, CostClass, ReplayCommandKindLabel, ReplayCommandManifestLabel,
        ReplayCycleReservePolicyLabel, ReplayImplementationStatus, ReplayPolicy,
        ReplayQuotaPolicyLabel,
    },
};

/// Canonical replay-policy rows for Fleet Subnet Root command variants.
pub const ROOT_COMMAND_REPLAY_POLICY_MANIFEST: &[CommandReplayPolicy] = &[
    replay_protected(
        "AcceptFunding",
        "fleet_root_funding.accept.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "ActivateFleetAdmission",
        "fleet_admission.activate_root.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "ActivateFundingPolicyRotation",
        "fleet_funding_policy_rotation.activate_root.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "AdoptStore",
        "root.adopt_store.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "BootstrapStore",
        "root.bootstrap_store.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    snapshot_convergent(
        "GetOrCreateDelegationProof",
        "auth.get_or_create_chain_key_delegation_proof.v1",
        CostClass::RootChainKeySigning,
        Some(ROOT_CHAIN_KEY_SIGNING_QUOTA_V1),
        None,
    ),
    snapshot_convergent(
        "HandoffPoolCanister",
        "canister_pool.handoff.v1",
        CostClass::None,
        None,
        None,
    ),
    snapshot_convergent(
        "ImportPoolCanister",
        "canister_pool.import.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    query_or_read_only("InspectCanister"),
    snapshot_convergent(
        "MaintainPool",
        "canister_pool.maintain.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    query_or_read_only("ObserveCanister"),
    replay_protected(
        "OpenFleetAdmission",
        "fleet_admission.open_root.v1",
        CostClass::None,
        None,
        None,
    ),
    response_idempotent(
        "PrepareAuthoritySnapshot",
        "authority_restore.prepare_snapshot.v1",
    ),
    response_idempotent(
        "PrepareComponentRegistry",
        "component_registry.prepare_root.v1",
    ),
    response_idempotent("PrepareFleetActivation", "fleet_activation.prepare.v1"),
    replay_protected(
        "PrepareFleetAdmission",
        "fleet_admission.prepare_root.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "PrepareFundingPolicyRotation",
        "fleet_funding_policy_rotation.prepare_root.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "PrepareRoleAttestation",
        "auth.prepare_role_attestation.v1",
        CostClass::RootCanisterSignaturePrepare,
        Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_or_read_only("PreviewCycleRefill"),
    replay_protected(
        "ProvisionChild",
        "root.provision_child.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    replay_protected(
        "ProvisionComponent",
        "root.provision_component.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    response_idempotent(
        "ProvisionComponents",
        "component_provisioning.accept_root_batch.v1",
    ),
    replay_protected(
        "ProvisionPeer",
        "root.provision_peer.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    monotonic_publish("PublishReleaseSet", "wasm_store.admin.v1"),
    replay_protected(
        "RefillCycles",
        "icp.refill.v1",
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    replay_protected(
        "RemoveComponent",
        "root.remove_component.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    replay_protected(
        "RemoveRoot",
        "root.remove_root.v1",
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    replay_protected(
        "RemoveSubtree",
        "root.remove_subtree.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    command_dispatch(
        "RespondCapability",
        "root.capability_rpc.v1",
        "root.capability.command_manifest.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    response_idempotent(
        "ResumeAuthoritySnapshot",
        "authority_restore.resume_snapshot.v1",
    ),
    response_idempotent("ResumeFleetActivation", "fleet_activation.resume.v1"),
    snapshot_convergent(
        "RetryPoolRefill",
        "canister_pool.retry_refill.v1",
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    snapshot_convergent(
        "RetryPoolReset",
        "canister_pool.retry_reset.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    response_idempotent("SetCyclesFunding", "fleet.set_cycles_funding.v1"),
    response_idempotent("SetFleetStatus", "fleet.set_status.v1"),
    response_idempotent(
        "SynchronizeComponentDirectories",
        "component_provisioning.synchronize_affected_directories.v1",
    ),
    replay_protected(
        "SynchronizeRegistry",
        "fleet_registry.synchronize_root.v1",
        CostClass::None,
        None,
        None,
    ),
    snapshot_convergent(
        "UpsertIssuerPolicy",
        "auth.upsert_root_issuer_policy.v1",
        CostClass::None,
        None,
        None,
    ),
    snapshot_convergent(
        "UpsertIssuerRenewalTemplate",
        "auth.upsert_root_issuer_renewal_template.v1",
        CostClass::None,
        None,
        None,
    ),
];

/// Canonical replay-policy rows for Fleet Coordinator command variants.
pub const COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST: &[CommandReplayPolicy] = &[
    response_idempotent(
        "AcknowledgeRootSnapshot",
        "fleet_registry.acknowledge_root.v1",
    ),
    response_idempotent("ActivateRegistry", "fleet_registry.activate.v1"),
    replay_protected(
        "ApplyFundingPolicyRotation",
        "fleet_funding_policy_rotation.apply.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "BeginFundingPolicyRotation",
        "fleet_funding_policy_rotation.begin.v1",
        CostClass::None,
        None,
        None,
    ),
    response_idempotent(
        "CompleteRootDeletion",
        "fleet_registry.complete_root_deletion.v1",
    ),
    response_idempotent("JoinRoot", "fleet_registry.root_join.v1"),
    replay_protected(
        "MutateAdmission",
        "fleet_admission.mutate.v1",
        CostClass::None,
        None,
        None,
    ),
    response_idempotent(
        "PrepareAuthoritySnapshot",
        "authority_restore.prepare_snapshot.v1",
    ),
    response_idempotent(
        "PrepareRootDeletionExecution",
        "fleet_registry.begin_root_deletion_execution.v1",
    ),
    replay_protected(
        "ProvisionComponents",
        "coordinator.provision_components.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "RemoveRoot",
        "coordinator.remove_root.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "RequestRootFunding",
        "fleet_root_funding.request.v1",
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    response_idempotent(
        "ResumeAuthoritySnapshot",
        "authority_restore.resume_snapshot.v1",
    ),
    response_idempotent("SetRootFunding", "fleet_root_funding.set_enabled.v1"),
    replay_protected(
        "StageFundingPolicyRotationRoot",
        "fleet_funding_policy_rotation.stage_root.v1",
        CostClass::None,
        None,
        None,
    ),
];

/// Canonical replay-policy rows for managed Canister command variants.
pub const MANAGED_COMMAND_REPLAY_POLICY_MANIFEST: &[CommandReplayPolicy] = &[
    replay_protected(
        "ActivateFleetAdmission",
        "fleet_admission.activate_target.v1",
        CostClass::None,
        None,
        None,
    ),
    response_idempotent("ApplicationSession", "auth.application_session.v1"),
    response_idempotent("ConfigureRuntime", "component_runtime.configure.v1"),
    intentionally_non_idempotent(
        "InstallDelegationProof",
        "auth.install_active_delegation_proof.v1",
        "controller maintenance replaces issuer-local active proof metadata",
    ),
    query_or_read_only("Observe"),
    replay_protected(
        "OpenFleetAdmission",
        "fleet_admission.open_target.v1",
        CostClass::None,
        None,
        None,
    ),
    replay_protected(
        "PrepareDelegatedToken",
        "auth.prepare_delegated_token.v1",
        CostClass::IssuerCanisterSignaturePrepare,
        Some(ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    replay_protected(
        "PrepareFleetAdmission",
        "fleet_admission.prepare_target.v1",
        CostClass::None,
        None,
        None,
    ),
    command_dispatch(
        "RespondCapability",
        "root.capability_rpc.v1",
        "root.capability.command_manifest.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
];

/// Canonical replay-policy rows for Wasm Store command variants.
pub const STORE_COMMAND_REPLAY_POLICY_MANIFEST: &[CommandReplayPolicy] = &[
    response_idempotent("ActivateFleet", "fleet_activation.activate.v1"),
    query_or_read_only("InspectTemplate"),
    monotonic_transition("PrepareChunkSet", "wasm_store.prepare.v1"),
    monotonic_transition(
        "PrepareFleetCredential",
        "fleet_activation.prepare_credential_generation.v1",
    ),
    snapshot_convergent(
        "ReclaimDeletionCycles",
        "wasm_store.reclaim_deletion_cycles.v1",
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    command_dispatch(
        "RespondCapability",
        "root.capability_rpc.v1",
        "root.capability.command_manifest.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    monotonic_transition("RunGc", "wasm_store.gc.v1"),
    monotonic_transition("StageManifest", "wasm_store.stage_manifest.v1"),
    snapshot_convergent(
        "SynchronizeState",
        "cascade.sync_state.v1",
        CostClass::None,
        None,
        None,
    ),
    snapshot_convergent(
        "SynchronizeTopology",
        "cascade.sync_topology.v1",
        CostClass::None,
        None,
        None,
    ),
];

/// Returns the canonical Root command replay-policy manifest.
#[must_use]
pub const fn root_command_replay_policy_manifest() -> &'static [CommandReplayPolicy] {
    ROOT_COMMAND_REPLAY_POLICY_MANIFEST
}

/// Returns the canonical Coordinator command replay-policy manifest.
#[must_use]
pub const fn coordinator_command_replay_policy_manifest() -> &'static [CommandReplayPolicy] {
    COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST
}

/// Returns the canonical managed Canister command replay-policy manifest.
#[must_use]
pub const fn managed_command_replay_policy_manifest() -> &'static [CommandReplayPolicy] {
    MANAGED_COMMAND_REPLAY_POLICY_MANIFEST
}

/// Returns the canonical Wasm Store command replay-policy manifest.
#[must_use]
pub const fn store_command_replay_policy_manifest() -> &'static [CommandReplayPolicy] {
    STORE_COMMAND_REPLAY_POLICY_MANIFEST
}

const fn command_kind(label: &'static str) -> ReplayCommandKindLabel {
    ReplayCommandKindLabel::new(label)
}

const fn response_idempotent(variant: &'static str, label: &'static str) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind(label),
        },
        CostClass::None,
        None,
        None,
    )
}

const fn query_or_read_only(variant: &'static str) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    )
}

const fn replay_protected(
    variant: &'static str,
    label: &'static str,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind(label),
            requires_operation_id: true,
        },
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    )
}

const fn snapshot_convergent(
    variant: &'static str,
    label: &'static str,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind(label),
        },
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    )
}

const fn monotonic_publish(variant: &'static str, label: &'static str) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind(label),
        },
        CostClass::DurablePublish,
        Some(DURABLE_PUBLISH_QUOTA_V1),
        Some(DURABLE_PUBLISH_RESERVE_V1),
    )
}

const fn monotonic_transition(variant: &'static str, label: &'static str) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind(label),
        },
        CostClass::None,
        None,
        None,
    )
}

const fn intentionally_non_idempotent(
    variant: &'static str,
    label: &'static str,
    reason: &'static str,
) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::IntentionallyNonIdempotent {
            command_kind: command_kind(label),
            reason,
        },
        CostClass::None,
        None,
        None,
    )
}

const fn command_dispatch(
    variant: &'static str,
    label: &'static str,
    manifest: &'static str,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> CommandReplayPolicy {
    command_policy(
        variant,
        ReplayPolicy::CommandDispatch {
            command_kind: command_kind(label),
            command_manifest: ReplayCommandManifestLabel::new(manifest),
        },
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    )
}

const fn command_policy(
    variant: &'static str,
    replay_policy: ReplayPolicy,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> CommandReplayPolicy {
    CommandReplayPolicy {
        variant,
        replay_policy,
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}
