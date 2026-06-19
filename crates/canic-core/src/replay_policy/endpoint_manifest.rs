//! Module: replay_policy::endpoint_manifest
//!
//! Responsibility: record replay policy for Canic-owned endpoint surfaces.
//! Does not own: endpoint routing, workflow execution, or replay receipt storage.
//! Boundary: endpoint manifest rows consumed by replay policy tests and workflows.

use crate::replay_policy::{
    quota::{
        DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1, DURABLE_PUBLISH_QUOTA_V1,
        DURABLE_PUBLISH_RESERVE_V1, ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1,
        ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1, VALUE_TRANSFER_QUOTA_V1,
        VALUE_TRANSFER_RESERVE_V1,
    },
    types::{
        CostClass, EndpointKind, EndpointReplayPolicy, ReplayImplementationStatus, ReplayPolicy,
    },
};

/// Canonical replay-policy rows for Canic endpoint methods.
pub const ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    update_response_idempotent("canic_app", "app.command.v1"),
    update_read_only("canic_canister_status"),
    update_costed_response_idempotent(
        "canic_canister_upgrade",
        "management.canister_upgrade.v1",
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_replay_protected(
        "canic_icp_refill",
        "icp.refill.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ValueTransfer,
        Some(VALUE_TRANSFER_QUOTA_V1),
        Some(VALUE_TRANSFER_RESERVE_V1),
    ),
    update_command_dispatch(
        "canic_pool_admin",
        "pool.admin.v1",
        "pool.admin.command_manifest.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_snapshot_convergent(
        "canic_upsert_root_issuer_policy",
        "auth.upsert_root_issuer_policy.v1",
    ),
    update_replay_protected(
        "canic_prepare_delegation_proof_batch",
        "auth.prepare_delegation_proof_batch.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::RootCanisterSignaturePrepare,
        Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_read_only("canic_get_delegation_proof_batch"),
    update_response_idempotent(
        "canic_install_delegation_proof_batch",
        "auth.install_delegation_proof_batch.v1",
    ),
    update_replay_protected(
        "canic_prepare_role_attestation",
        "auth.prepare_role_attestation.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::RootCanisterSignaturePrepare,
        Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_read_only("canic_get_role_attestation"),
    update_command_dispatch(
        "canic_response_capability_v1",
        "root.capability_rpc.v1",
        "root.capability.command_manifest.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    update_snapshot_convergent("canic_sync_state", "cascade.sync_state.v1"),
    update_snapshot_convergent("canic_sync_topology", "cascade.sync_topology.v1"),
    update_intentionally_non_idempotent(
        "canic_install_active_delegation_proof",
        "auth.install_active_delegation_proof.v1",
        "controller maintenance endpoint replaces issuer-local active proof metadata",
    ),
    query_read_only("canic_active_delegation_proof_status"),
    update_replay_protected(
        "canic_prepare_delegated_token",
        "auth.prepare_delegated_token.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::IssuerCanisterSignaturePrepare,
        Some(ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_read_only("canic_get_delegated_token"),
    update_monotonic_publish(
        "canic_template_prepare_admin",
        "wasm_store.template_prepare_admin.v1",
    ),
    update_monotonic_publish(
        "canic_template_publish_chunk_admin",
        "wasm_store.template_publish_chunk_admin.v1",
    ),
    update_monotonic_publish(
        "canic_template_stage_manifest_admin",
        "wasm_store.template_stage_manifest_admin.v1",
    ),
    update_response_idempotent(
        "canic_wasm_store_bootstrap_resume_root_admin",
        "wasm_store.bootstrap_resume.ensure_v1",
    ),
    update_monotonic_publish("canic_wasm_store_admin", "wasm_store.admin.v1"),
    update_monotonic_publish("canic_wasm_store_begin_gc", "wasm_store.begin_gc.v1"),
    update_monotonic_publish("canic_wasm_store_chunk", "wasm_store.chunk.v1"),
    update_monotonic_publish("canic_wasm_store_complete_gc", "wasm_store.complete_gc.v1"),
    update_monotonic_publish("canic_wasm_store_info", "wasm_store.info.v1"),
    update_monotonic_publish("canic_wasm_store_prepare", "wasm_store.prepare.v1"),
    update_monotonic_publish("canic_wasm_store_prepare_gc", "wasm_store.prepare_gc.v1"),
    update_monotonic_publish(
        "canic_wasm_store_publish_chunk",
        "wasm_store.publish_chunk.v1",
    ),
    update_monotonic_publish(
        "canic_wasm_store_stage_manifest",
        "wasm_store.stage_manifest.v1",
    ),
];

/// Returns the canonical endpoint replay-policy manifest.
#[must_use]
pub const fn endpoint_replay_policy_manifest() -> &'static [EndpointReplayPolicy] {
    ENDPOINT_REPLAY_POLICY_MANIFEST
}

const fn update_response_idempotent(
    endpoint: &'static str,
    command_kind: &'static str,
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
    command_kind: &'static str,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
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
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
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
    command_kind: &'static str,
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

const fn update_snapshot_convergent(
    endpoint: &'static str,
    command_kind: &'static str,
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

const fn update_intentionally_non_idempotent(
    endpoint: &'static str,
    command_kind: &'static str,
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
    command_kind: &'static str,
    command_manifest: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
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
