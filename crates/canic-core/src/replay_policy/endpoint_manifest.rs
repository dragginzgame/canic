//! Module: replay_policy::endpoint_manifest
//!
//! Responsibility: record replay policy for Canic-owned endpoint surfaces.
//! Does not own: endpoint routing, workflow execution, or replay receipt storage.
//! Boundary: endpoint manifest rows consumed by replay policy tests and workflows.

use crate::replay_policy::{
    quota::{
        DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1, DURABLE_PUBLISH_QUOTA_V1,
        DURABLE_PUBLISH_RESERVE_V1, ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1,
        ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1, ROOT_CHAIN_KEY_SIGNING_QUOTA_V1,
        VALUE_TRANSFER_QUOTA_V1, VALUE_TRANSFER_RESERVE_V1,
    },
    types::{
        CostClass, EndpointKind, EndpointReplayPolicy, ReplayCommandKindLabel,
        ReplayCommandManifestLabel, ReplayCycleReservePolicyLabel, ReplayImplementationStatus,
        ReplayPolicy, ReplayQuotaPolicyLabel,
    },
};

/// Canonical replay-policy rows for Canic endpoint methods.
pub const ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    query_read_only("canic_authority_restore_fence_status"),
    update_response_idempotent(
        "canic_authority_snapshot_prepare",
        command_kind("authority_restore.prepare_snapshot.v1"),
    ),
    update_response_idempotent(
        "canic_authority_snapshot_resume",
        command_kind("authority_restore.resume_snapshot.v1"),
    ),
    update_response_idempotent("canic_fleet_admin", command_kind("fleet.command.v1")),
    update_read_only("canic_canister_status"),
    query_read_only("canic_pool_list"),
    update_costed_snapshot_convergent(
        "canic_pool_admin",
        command_kind("canister_pool.maintain.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_replay_protected(
        "canic_icp_refill",
        command_kind("icp.refill.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    update_snapshot_convergent(
        "canic_upsert_root_issuer_policy",
        command_kind("auth.upsert_root_issuer_policy.v1"),
    ),
    update_snapshot_convergent(
        "canic_upsert_root_issuer_renewal_template",
        command_kind("auth.upsert_root_issuer_renewal_template.v1"),
    ),
    update_costed_snapshot_convergent(
        "canic_get_or_create_chain_key_delegation_proof",
        command_kind("auth.get_or_create_chain_key_delegation_proof.v1"),
        CostClass::RootChainKeySigning,
        Some(ROOT_CHAIN_KEY_SIGNING_QUOTA_V1),
        None,
    ),
    update_replay_protected(
        "canic_prepare_role_attestation",
        command_kind("auth.prepare_role_attestation.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::RootCanisterSignaturePrepare,
        Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_read_only("canic_get_role_attestation"),
    update_command_dispatch(
        "canic_response_capability_v1",
        command_kind("root.capability_rpc.v1"),
        command_manifest("root.capability.command_manifest.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_snapshot_convergent("canic_sync_state", command_kind("cascade.sync_state.v1")),
    update_snapshot_convergent(
        "canic_sync_topology",
        command_kind("cascade.sync_topology.v1"),
    ),
    update_response_idempotent(
        "canic_prepare_fleet_activation",
        command_kind("fleet_activation.prepare.v1"),
    ),
    update_response_idempotent(
        "canic_resume_fleet_activation",
        command_kind("fleet_activation.resume.v1"),
    ),
    update_response_idempotent(
        "canic_root_store_bootstrap",
        command_kind("fleet.root_store_bootstrap.v1"),
    ),
    query_read_only("canic_root_store_bootstrap_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_join",
        command_kind("fleet_registry.root_join.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_registry_acknowledge_root",
        command_kind("fleet_registry.acknowledge_root.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_registry_activate",
        command_kind("fleet_registry.activate.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_component_provisioning_prepare",
        command_kind("fleet_component_provisioning.prepare.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_component_provisioning_advance",
        command_kind("fleet_component_provisioning.advance.v1"),
    ),
    query_read_only("canic_fleet_component_provisioning_status"),
    update_response_idempotent(
        "canic_fleet_registry_root_draining_reservation_prepare",
        command_kind("fleet_registry.reserve_root_draining.v1"),
    ),
    update_read_only("canic_fleet_registry_root_draining_reservation_status"),
    update_response_idempotent(
        "canic_fleet_registry_publish_root_draining",
        command_kind("fleet_registry.publish_root_draining.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_registry_publish_root_removed",
        command_kind("fleet_registry.publish_root_removed.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_registry_root_deletion_readiness_prepare",
        command_kind("fleet_registry.prepare_root_deletion_readiness.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_registry_root_deletion_ready",
        command_kind("fleet_registry.record_root_deletion_readiness.v1"),
    ),
    update_response_idempotent(
        "canic_fleet_registry_root_deletion_execution_begin",
        command_kind("fleet_registry.begin_root_deletion_execution.v1"),
    ),
    query_read_only("canic_fleet_registry_root_deletion_execution_status"),
    update_response_idempotent(
        "canic_fleet_registry_root_deletion_complete",
        command_kind("fleet_registry.complete_root_deletion.v1"),
    ),
    query_read_only("canic_fleet_registry_root_deletion_status"),
    update_read_only("canic_fleet_registry_snapshot_for_root"),
    update_response_idempotent(
        "canic_fleet_registry_synchronize",
        command_kind("fleet_registry.synchronize_root.v1"),
    ),
    update_snapshot_convergent(
        "canic_fleet_registry_activate_mirror",
        command_kind("fleet_registry.activate_root_mirror.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_registry_prepare",
        command_kind("component_registry.prepare_root.v1"),
    ),
    query_read_only("canic_root_component_registry_status"),
    update_response_idempotent(
        "canic_root_component_provisioning_accept",
        command_kind("component_provisioning.accept_root_batch.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_provisioning_advance",
        command_kind("component_provisioning.advance_root_batch.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_provisioning_publish",
        command_kind("component_provisioning.publish_root_batch.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_directories_synchronize",
        command_kind("component_provisioning.synchronize_affected_directories.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_provisioning_activate",
        command_kind("component_provisioning.activate_root_batch.v1"),
    ),
    query_read_only("canic_root_component_provisioning_status"),
    query_read_only("canic_fleet_subnet_root_canister_summary"),
    update_response_idempotent(
        "canic_fleet_subnet_wasm_store_adopt",
        command_kind("fleet_subnet_root.adopt_wasm_store.v1"),
    ),
    query_read_only("canic_fleet_subnet_wasm_store_adoption_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_draining_begin",
        command_kind("fleet_subnet_root.begin_draining.v1"),
    ),
    query_read_only("canic_fleet_subnet_root_draining_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_removal_publish",
        command_kind("fleet_subnet_root.publish_removal.v1"),
    ),
    query_read_only("canic_fleet_subnet_root_removal_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_store_reclaim",
        command_kind("fleet_subnet_root.reclaim_store.v1"),
    ),
    query_read_only("canic_fleet_subnet_root_store_reclamation_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_store_binding_finalize",
        command_kind("fleet_subnet_root.finalize_store_binding.v1"),
    ),
    query_read_only("canic_fleet_subnet_root_store_binding_finalization_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_store_delete",
        command_kind("fleet_subnet_root.delete_store.v1"),
    ),
    query_read_only("canic_fleet_subnet_root_store_deletion_status"),
    update_costed_response_idempotent(
        "canic_fleet_subnet_root_deletion_prepare",
        command_kind("fleet_subnet_root.prepare_deletion.v1"),
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    query_read_only("canic_fleet_subnet_root_deletion_preparation_status"),
    update_response_idempotent(
        "canic_fleet_subnet_root_draining_inventory_finalize",
        command_kind("fleet_subnet_root.finalize_inventory.v1"),
    ),
    query_read_only("canic_fleet_subnet_root_draining_inventory_status"),
    update_response_idempotent(
        "canic_root_component_allocate",
        command_kind("component_registry.allocate_top_level.v1"),
    ),
    update_response_idempotent(
        "canic_root_peer_component_allocate",
        command_kind("component_registry.allocate_peer.v1"),
    ),
    query_read_only("canic_root_peer_component_allocation_status"),
    query_read_only("canic_root_component_allocation_status"),
    update_response_idempotent(
        "canic_root_component_child_allocate",
        command_kind("component_registry.allocate_child.v1"),
    ),
    query_read_only("canic_root_component_child_allocation_status"),
    update_response_idempotent(
        "canic_root_component_draining_begin",
        command_kind("component_registry.begin_component_draining.v1"),
    ),
    query_read_only("canic_root_component_draining_status"),
    update_response_idempotent(
        "canic_root_component_quiesce",
        command_kind("component_registry.quiesce_component.v1"),
    ),
    query_read_only("canic_root_component_quiescence_status"),
    update_snapshot_convergent(
        "canic_root_component_draining_advance",
        command_kind("component_registry.advance_component_draining.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_draining_inventory_finalize",
        command_kind("component_registry.finalize_component_inventory.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_delete",
        command_kind("management.control_plane.component_delete.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_membership_remove",
        command_kind("component_registry.remove_component_membership.v1"),
    ),
    query_read_only("canic_root_component_deletion_status"),
    update_response_idempotent(
        "canic_root_component_subtree_removal_begin",
        command_kind("component_registry.begin_subtree_removal.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_advance",
        command_kind("component_registry.advance_subtree_removal.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_stop_prepare",
        command_kind("component_registry.prepare_subtree_stop.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_stop",
        command_kind("management.control_plane.component_subtree_stop.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_delete_prepare",
        command_kind("component_registry.prepare_subtree_delete.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_delete",
        command_kind("management.control_plane.component_subtree_delete.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_membership_remove",
        command_kind("component_registry.remove_subtree_leaf_membership.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_directory_synchronize",
        command_kind("component_registry.synchronize_subtree_removal_directory.v1"),
    ),
    update_snapshot_convergent(
        "canic_root_component_subtree_removal_leaf_finalize",
        command_kind("component_registry.finalize_subtree_removal_leaf.v1"),
    ),
    query_read_only("canic_root_component_subtree_removal_status"),
    update_costed_response_idempotent(
        "canic_root_component_child_create",
        command_kind("management.control_plane.component_child_create.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_costed_response_idempotent(
        "canic_root_component_child_install",
        command_kind("management.control_plane.component_child_install.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_response_idempotent(
        "canic_root_component_child_commit",
        command_kind("component_registry.commit_child.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_child_directory_prepare",
        command_kind("component_registry.prepare_child_directory.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_child_runtime_activate",
        command_kind("component_registry.activate_child_runtime.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_child_membership_activate",
        command_kind("component_registry.activate_child_membership.v1"),
    ),
    update_costed_response_idempotent(
        "canic_root_component_create",
        command_kind("management.control_plane.component_create.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_costed_response_idempotent(
        "canic_root_peer_component_create",
        command_kind("management.control_plane.peer_component_create.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_costed_response_idempotent(
        "canic_root_component_install",
        command_kind("management.control_plane.component_install.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_costed_response_idempotent(
        "canic_root_peer_component_install",
        command_kind("management.control_plane.peer_component_install.v1"),
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_response_idempotent(
        "canic_root_component_commit",
        command_kind("component_registry.commit_top_level.v1"),
    ),
    update_response_idempotent(
        "canic_root_peer_component_commit",
        command_kind("component_registry.commit_peer.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_directory_prepare",
        command_kind("component_registry.prepare_component_directory.v1"),
    ),
    update_response_idempotent(
        "canic_root_peer_component_directory_prepare",
        command_kind("component_registry.prepare_peer_directory.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_runtime_activate",
        command_kind("component_registry.activate_component_runtime.v1"),
    ),
    update_response_idempotent(
        "canic_root_peer_component_runtime_activate",
        command_kind("component_registry.activate_peer_runtime.v1"),
    ),
    update_response_idempotent(
        "canic_root_component_membership_activate",
        command_kind("component_registry.activate_component_membership.v1"),
    ),
    update_response_idempotent(
        "canic_root_peer_component_membership_activate",
        command_kind("component_registry.activate_peer_membership.v1"),
    ),
    query_read_only("canic_root_component_registry_partition"),
    query_read_only("canic_root_component_directory_head"),
    query_read_only("canic_root_component_directory_page"),
    update_response_idempotent(
        "canic_component_runtime_directory_prepare",
        command_kind("component_runtime.prepare_directory.v1"),
    ),
    update_response_idempotent(
        "canic_component_runtime_directory_synchronize",
        command_kind("component_runtime.synchronize_directory.v1"),
    ),
    query_read_only("canic_component_runtime_status"),
    update_response_idempotent(
        "canic_component_runtime_activate",
        command_kind("component_runtime.activate.v1"),
    ),
    update_monotonic_transition(
        "canic_prepare_fleet_credential_generation",
        command_kind("fleet_activation.prepare_credential_generation.v1"),
    ),
    update_response_idempotent(
        "canic_activate_fleet",
        command_kind("fleet_activation.activate.v1"),
    ),
    update_intentionally_non_idempotent(
        "canic_install_active_delegation_proof",
        command_kind("auth.install_active_delegation_proof.v1"),
        "controller maintenance endpoint replaces issuer-local active proof metadata",
    ),
    query_read_only("canic_active_delegation_proof_status"),
    update_replay_protected(
        "canic_prepare_delegated_token",
        command_kind("auth.prepare_delegated_token.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::IssuerCanisterSignaturePrepare,
        Some(ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_read_only("canic_get_delegated_token"),
    query_read_only("canic_health"),
    query_read_only("canic_readiness"),
    query_read_only("canic_runtime_status"),
    query_read_only("canic_managed_canister_binding"),
    update_monotonic_transition(
        "canic_template_prepare_admin",
        command_kind("wasm_store.template_prepare_admin.v1"),
    ),
    update_monotonic_transition(
        "canic_template_publish_chunk_admin",
        command_kind("wasm_store.template_publish_chunk_admin.v1"),
    ),
    update_monotonic_transition(
        "canic_template_stage_manifest_admin",
        command_kind("wasm_store.template_stage_manifest_admin.v1"),
    ),
    update_monotonic_publish(
        "canic_wasm_store_admin",
        command_kind("wasm_store.admin.v1"),
    ),
    update_monotonic_transition(
        "canic_wasm_store_begin_gc",
        command_kind("wasm_store.begin_gc.v1"),
    ),
    update_monotonic_transition(
        "canic_wasm_store_chunk",
        command_kind("wasm_store.chunk.v1"),
    ),
    update_monotonic_transition(
        "canic_wasm_store_complete_gc",
        command_kind("wasm_store.complete_gc.v1"),
    ),
    update_monotonic_transition("canic_wasm_store_info", command_kind("wasm_store.info.v1")),
    update_monotonic_transition(
        "canic_wasm_store_prepare",
        command_kind("wasm_store.prepare.v1"),
    ),
    update_monotonic_transition(
        "canic_wasm_store_prepare_gc",
        command_kind("wasm_store.prepare_gc.v1"),
    ),
    update_costed_snapshot_convergent(
        "canic_wasm_store_reclaim_deletion_cycles",
        command_kind("wasm_store.reclaim_deletion_cycles.v1"),
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    update_monotonic_transition(
        "canic_wasm_store_publish_chunk",
        command_kind("wasm_store.publish_chunk.v1"),
    ),
    update_monotonic_transition(
        "canic_wasm_store_stage_manifest",
        command_kind("wasm_store.stage_manifest.v1"),
    ),
];

/// Returns the canonical endpoint replay-policy manifest.
#[must_use]
pub const fn endpoint_replay_policy_manifest() -> &'static [EndpointReplayPolicy] {
    ENDPOINT_REPLAY_POLICY_MANIFEST
}

const fn command_kind(label: &'static str) -> ReplayCommandKindLabel {
    ReplayCommandKindLabel::new(label)
}

const fn command_manifest(label: &'static str) -> ReplayCommandManifestLabel {
    ReplayCommandManifestLabel::new(label)
}

const fn update_response_idempotent(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::ResponseIdempotent { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn update_costed_response_idempotent(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::ResponseIdempotent { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}

const fn update_read_only(endpoint: &'static str) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::QueryOrReadOnly,
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn query_read_only(endpoint: &'static str) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Query,
        replay_policy: ReplayPolicy::QueryOrReadOnly,
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn update_replay_protected(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
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

const fn update_monotonic_publish(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::MonotonicTransition { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::DurablePublish,
        quota_policy: Some(DURABLE_PUBLISH_QUOTA_V1),
        cycle_reserve_policy: Some(DURABLE_PUBLISH_RESERVE_V1),
    }
}

const fn update_monotonic_transition(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::MonotonicTransition { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn update_snapshot_convergent(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::SnapshotConvergent { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn update_costed_snapshot_convergent(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::SnapshotConvergent { command_kind },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}

const fn update_intentionally_non_idempotent(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
    reason: &'static str,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::IntentionallyNonIdempotent {
            command_kind,
            reason,
        },
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn update_command_dispatch(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
    command_manifest: ReplayCommandManifestLabel,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::CommandDispatch {
            command_kind,
            command_manifest,
        },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}
