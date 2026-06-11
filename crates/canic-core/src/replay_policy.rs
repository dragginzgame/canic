//! Replay policy inventory for Canic-owned endpoint surfaces.
//!
//! This is Slice A scaffolding for the 0.61 replay-safety work. It records the
//! intended replay and cost policy for endpoints emitted by Canic macros. Later
//! slices wire these classifications into shared replay receipts and cost
//! guards.

///
/// EndpointKind
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointKind {
    Query,
    Update,
}

///
/// ReplayPolicy
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayPolicy {
    QueryOrReadOnly,
    ResponseIdempotent {
        command_kind: &'static str,
    },
    ReplayProtected {
        command_kind: &'static str,
        requires_operation_id: bool,
    },
    MonotonicTransition {
        command_kind: &'static str,
    },
    SnapshotConvergent {
        command_kind: &'static str,
    },
    CommandDispatch {
        command_kind: &'static str,
        command_manifest: &'static str,
    },
    IntentionallyNonIdempotent {
        command_kind: &'static str,
        reason: &'static str,
    },
}

///
/// CostClass
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostClass {
    None,
    RootCanisterSignaturePrepare,
    IssuerCanisterSignaturePrepare,
    ManagementDeployment,
    ValueTransfer,
    DurablePublish,
}

///
/// ReplayImplementationStatus
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayImplementationStatus {
    Implemented,
    ReleaseBlocker,
}

///
/// EndpointReplayPolicy
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EndpointReplayPolicy {
    pub endpoint: &'static str,
    pub endpoint_kind: EndpointKind,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<&'static str>,
    pub cycle_reserve_policy: Option<&'static str>,
}

///
/// PoolAdminCommandReplayPolicy
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolAdminCommandReplayPolicy {
    pub variant: &'static str,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<&'static str>,
    pub cycle_reserve_policy: Option<&'static str>,
}

///
/// RootCapabilityCommandReplayPolicy
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootCapabilityCommandReplayPolicy {
    pub variant: &'static str,
    pub replay_policy: ReplayPolicy,
    pub implementation_status: ReplayImplementationStatus,
    pub cost_class: CostClass,
    pub quota_policy: Option<&'static str>,
    pub cycle_reserve_policy: Option<&'static str>,
}

const ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1: &str = "root_canister_signature_prepare.quota.v1";
const ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1: &str =
    "issuer_canister_signature_prepare.quota.v1";
const DEPLOYMENT_QUOTA_V1: &str = "deployment.quota.v1";
const DEPLOYMENT_RESERVE_V1: &str = "deployment.cycle_reserve.v1";
const VALUE_TRANSFER_QUOTA_V1: &str = "value_transfer.quota.v1";
const VALUE_TRANSFER_RESERVE_V1: &str = "value_transfer.cycle_reserve.v1";
const DURABLE_PUBLISH_QUOTA_V1: &str = "durable_publish.quota.v1";
const DURABLE_PUBLISH_RESERVE_V1: &str = "durable_publish.cycle_reserve.v1";

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
    update_replay_protected(
        "canic_prepare_delegation_proof",
        "auth.prepare_delegation_proof.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::RootCanisterSignaturePrepare,
        Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
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

pub const POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST: &[PoolAdminCommandReplayPolicy] = &[
    pool_admin_replay_protected(
        "CreateEmpty",
        "pool.create_empty.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    pool_admin_response_idempotent(
        "Recycle",
        "pool.recycle.ensure_v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    pool_admin_response_idempotent(
        "ImportImmediate",
        "pool.import_immediate.ensure_v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    pool_admin_snapshot_convergent(
        "ImportQueued",
        "pool.import_queued.ensure_v1",
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
];

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

#[must_use]
pub const fn endpoint_replay_policy_manifest() -> &'static [EndpointReplayPolicy] {
    ENDPOINT_REPLAY_POLICY_MANIFEST
}

#[must_use]
pub const fn pool_admin_command_replay_policy_manifest() -> &'static [PoolAdminCommandReplayPolicy]
{
    POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
}

#[must_use]
pub const fn root_capability_command_replay_policy_manifest()
-> &'static [RootCapabilityCommandReplayPolicy] {
    ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
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

const fn pool_admin_response_idempotent(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> PoolAdminCommandReplayPolicy {
    PoolAdminCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::ResponseIdempotent { command_kind },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}

const fn pool_admin_replay_protected(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> PoolAdminCommandReplayPolicy {
    PoolAdminCommandReplayPolicy {
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

const fn pool_admin_snapshot_convergent(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> PoolAdminCommandReplayPolicy {
    PoolAdminCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::SnapshotConvergent { command_kind },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        CANIC_TEMPLATE_PREPARE_ADMIN, CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
        CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN, CANIC_WASM_STORE_ROOT_UPDATE_METHODS,
    };
    use std::collections::BTreeSet;

    #[test]
    fn endpoint_manifest_entries_are_unique() {
        let mut seen = BTreeSet::new();
        for entry in ENDPOINT_REPLAY_POLICY_MANIFEST {
            assert!(
                seen.insert(entry.endpoint),
                "duplicate replay policy entry for {}",
                entry.endpoint
            );
        }
    }

    #[test]
    fn emitted_canic_update_endpoints_have_replay_policy_entries() {
        let emitted = emitted_update_endpoint_names();
        let manifest = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| entry.endpoint_kind == EndpointKind::Update)
            .map(|entry| entry.endpoint)
            .collect::<BTreeSet<_>>();

        let missing = emitted.difference(&manifest).copied().collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "missing replay policy entries for update endpoints: {missing:?}"
        );
    }

    #[test]
    fn costed_manifest_entries_declare_guards() {
        for entry in ENDPOINT_REPLAY_POLICY_MANIFEST {
            if entry.cost_class == CostClass::None {
                continue;
            }
            assert!(
                entry.quota_policy.is_some(),
                "costed entry {} missing quota policy",
                entry.endpoint
            );
            assert!(
                entry.cost_class == CostClass::RootCanisterSignaturePrepare
                    || entry.cost_class == CostClass::IssuerCanisterSignaturePrepare
                    || entry.cycle_reserve_policy.is_some(),
                "costed entry {} missing cycle-reserve policy",
                entry.endpoint
            );
        }
    }

    #[test]
    fn costed_pool_admin_command_entries_declare_guards() {
        for entry in POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST {
            if entry.cost_class == CostClass::None {
                continue;
            }
            assert!(
                entry.quota_policy.is_some(),
                "costed pool admin command {} missing quota policy",
                entry.variant
            );
            assert!(
                entry.cycle_reserve_policy.is_some(),
                "costed pool admin command {} missing cycle-reserve policy",
                entry.variant
            );
        }
    }

    #[test]
    fn costed_root_capability_command_entries_declare_guards() {
        for entry in ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST {
            if entry.cost_class == CostClass::None {
                continue;
            }
            assert!(
                entry.quota_policy.is_some(),
                "costed root capability command {} missing quota policy",
                entry.variant
            );
            assert!(
                entry.cycle_reserve_policy.is_some(),
                "costed root capability command {} missing cycle-reserve policy",
                entry.variant
            );
        }
    }

    #[test]
    fn release_candidate_manifests_have_no_release_blockers() {
        let blockers = release_candidate_manifest_blockers();

        assert!(
            blockers.is_empty(),
            "release candidate manifests still contain replay blockers: {blockers:?}"
        );
    }

    #[test]
    fn durable_publish_entries_are_wasm_store_publication_surfaces() {
        let expected = durable_publish_endpoint_names();
        let actual = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| entry.cost_class == CostClass::DurablePublish)
            .map(|entry| entry.endpoint)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            actual, expected,
            "durable-publish cost class must stay scoped to wasm-store publication surfaces"
        );

        for endpoint in expected {
            let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
                .iter()
                .find(|entry| entry.endpoint == endpoint)
                .expect("durable publish endpoint entry");

            assert_eq!(
                entry.implementation_status,
                ReplayImplementationStatus::Implemented
            );
            assert_eq!(entry.endpoint_kind, EndpointKind::Update);
            assert!(
                matches!(
                    entry.replay_policy,
                    ReplayPolicy::MonotonicTransition { .. }
                ),
                "{endpoint} must stay classified as monotonic publication"
            );
            assert_eq!(entry.quota_policy, Some(DURABLE_PUBLISH_QUOTA_V1));
            assert_eq!(entry.cycle_reserve_policy, Some(DURABLE_PUBLISH_RESERVE_V1));
        }
    }

    #[test]
    fn delegation_proof_prepare_is_manifested_as_implemented() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_prepare_delegation_proof")
            .expect("delegation prepare endpoint policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.cost_class, CostClass::RootCanisterSignaturePrepare);
        assert_eq!(
            entry.quota_policy,
            Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1)
        );
        assert_eq!(entry.cycle_reserve_policy, None);
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: "auth.prepare_delegation_proof.v1",
                requires_operation_id: true,
            }
        );
    }

    #[test]
    fn active_delegation_proof_install_is_controller_maintenance() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_install_active_delegation_proof")
            .expect("active delegation proof install policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::IntentionallyNonIdempotent {
                command_kind: "auth.install_active_delegation_proof.v1",
                reason: "controller maintenance endpoint replaces issuer-local active proof metadata",
            }
        );
    }

    #[test]
    fn delegated_token_prepare_get_endpoints_are_manifested() {
        let prepare = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_prepare_delegated_token")
            .expect("delegated-token prepare policy entry");

        assert_eq!(
            prepare.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            prepare.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: "auth.prepare_delegated_token.v1",
                requires_operation_id: true,
            }
        );
        assert_eq!(
            prepare.cost_class,
            CostClass::IssuerCanisterSignaturePrepare
        );
        assert_eq!(
            prepare.quota_policy,
            Some(ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1)
        );
        assert_eq!(prepare.cycle_reserve_policy, None);

        let get = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_get_delegated_token")
            .expect("delegated-token get policy entry");

        assert_eq!(get.endpoint_kind, EndpointKind::Query);
        assert_eq!(get.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(get.cost_class, CostClass::None);
    }

    #[test]
    fn role_attestation_prepare_get_endpoints_are_manifested() {
        let prepare = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_prepare_role_attestation")
            .expect("role-attestation prepare policy entry");

        assert_eq!(
            prepare.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            prepare.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: "auth.prepare_role_attestation.v1",
                requires_operation_id: true,
            }
        );
        assert_eq!(prepare.cost_class, CostClass::RootCanisterSignaturePrepare);
        assert_eq!(
            prepare.quota_policy,
            Some(ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1)
        );
        assert_eq!(prepare.cycle_reserve_policy, None);

        let get = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_get_role_attestation")
            .expect("role-attestation get policy entry");

        assert_eq!(get.endpoint_kind, EndpointKind::Query);
        assert_eq!(get.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(get.cost_class, CostClass::None);
    }

    #[test]
    fn canister_status_is_manifested_as_read_only() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_canister_status")
            .expect("canister status policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
    }

    #[test]
    fn canister_upgrade_is_manifested_as_implemented_response_idempotent() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_canister_upgrade")
            .expect("canister upgrade policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ResponseIdempotent {
                command_kind: "management.canister_upgrade.v1",
            }
        );
        assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
        assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
    }

    #[test]
    fn icp_refill_is_manifested_as_implemented_value_transfer() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_icp_refill")
            .expect("ICP refill policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: "icp.refill.v1",
                requires_operation_id: true,
            }
        );
        assert_eq!(entry.cost_class, CostClass::ValueTransfer);
        assert_eq!(entry.quota_policy, Some(VALUE_TRANSFER_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(VALUE_TRANSFER_RESERVE_V1));
    }

    #[test]
    fn remaining_release_blockers_are_explicit_endpoint_slices() {
        let blockers = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| {
                entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker
            })
            .map(|entry| entry.endpoint)
            .collect::<BTreeSet<_>>();

        assert!(blockers.is_empty(), "unexpected blockers: {blockers:?}");
    }

    #[test]
    fn root_capability_command_variants_have_replay_policy_entries() {
        let variants = root_capability_command_variant_names();
        let manifest = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .map(|entry| entry.variant)
            .collect::<BTreeSet<_>>();

        assert_eq!(manifest, variants);
    }

    #[test]
    fn root_capability_endpoint_is_manifested_as_command_dispatch() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_response_capability_v1")
            .expect("root capability endpoint policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::CommandDispatch {
                command_kind: "root.capability_rpc.v1",
                command_manifest: "root.capability.command_manifest.v1",
            }
        );
        assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
        assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
    }

    #[test]
    fn root_capability_command_blockers_are_explicit() {
        let blockers = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| {
                entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker
            })
            .map(|entry| entry.variant)
            .collect::<BTreeSet<_>>();

        assert!(blockers.is_empty(), "unexpected blockers: {blockers:?}");
    }

    #[test]
    fn root_capability_implemented_commands_are_replay_protected() {
        for (variant, command_kind, cost_class) in [
            (
                "ProvisionCanister",
                "root.provision.v1",
                CostClass::ManagementDeployment,
            ),
            (
                "UpgradeCanister",
                "root.upgrade.v1",
                CostClass::ManagementDeployment,
            ),
            (
                "RecycleCanister",
                "root.recycle_canister.v1",
                CostClass::ManagementDeployment,
            ),
            (
                "RequestCycles",
                "root.request_cycles.v1",
                CostClass::ValueTransfer,
            ),
        ] {
            let entry = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
                .iter()
                .find(|entry| entry.variant == variant)
                .expect("root capability command policy entry");

            assert_eq!(
                entry.implementation_status,
                ReplayImplementationStatus::Implemented
            );
            assert_eq!(
                entry.replay_policy,
                ReplayPolicy::ReplayProtected {
                    command_kind,
                    requires_operation_id: true,
                }
            );
            assert_eq!(entry.cost_class, cost_class);
        }
    }

    #[test]
    fn pool_admin_command_variants_have_replay_policy_entries() {
        let variants = pool_admin_command_variant_names();
        let manifest = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .map(|entry| entry.variant)
            .collect::<BTreeSet<_>>();

        assert_eq!(manifest, variants);
    }

    #[test]
    fn pool_admin_endpoint_is_manifested_as_implemented_command_dispatch() {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == "canic_pool_admin")
            .expect("pool admin endpoint policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::CommandDispatch {
                command_kind: "pool.admin.v1",
                command_manifest: "pool.admin.command_manifest.v1",
            }
        );
        assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
        assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
    }

    #[test]
    fn pool_admin_endpoint_requires_all_command_variants_implemented() {
        let blockers = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| {
                entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker
            })
            .map(|entry| entry.variant)
            .collect::<Vec<_>>();

        assert!(
            blockers.is_empty(),
            "pool admin endpoint cannot be implemented while command variants remain blocked: {blockers:?}"
        );
    }

    #[test]
    fn pool_create_empty_command_is_manifested_as_implemented() {
        let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.variant == "CreateEmpty")
            .expect("CreateEmpty command policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: "pool.create_empty.v1",
                requires_operation_id: true,
            }
        );
    }

    #[test]
    fn pool_import_queued_command_is_manifested_as_implemented_convergent() {
        let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.variant == "ImportQueued")
            .expect("ImportQueued command policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::SnapshotConvergent {
                command_kind: "pool.import_queued.ensure_v1",
            }
        );
    }

    #[test]
    fn pool_import_immediate_command_is_manifested_as_implemented_idempotent() {
        let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.variant == "ImportImmediate")
            .expect("ImportImmediate command policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ResponseIdempotent {
                command_kind: "pool.import_immediate.ensure_v1",
            }
        );
        assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
        assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
    }

    #[test]
    fn pool_recycle_command_is_manifested_as_implemented_idempotent() {
        let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.variant == "Recycle")
            .expect("Recycle command policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ResponseIdempotent {
                command_kind: "pool.recycle.ensure_v1",
            }
        );
        assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
        assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
    }

    #[test]
    fn intentionally_non_idempotent_entries_must_state_reason() {
        for entry in ENDPOINT_REPLAY_POLICY_MANIFEST {
            if let ReplayPolicy::IntentionallyNonIdempotent { reason, .. } = entry.replay_policy {
                assert!(
                    !reason.trim().is_empty(),
                    "non-idempotent entry {} must state a reason",
                    entry.endpoint
                );
            }
        }
    }

    fn pool_admin_command_variant_names() -> BTreeSet<&'static str> {
        enum_variant_names_from_source(include_str!("dto/pool.rs"), "pub enum PoolAdminCommand")
    }

    fn root_capability_command_variant_names() -> BTreeSet<&'static str> {
        enum_variant_names_from_source(include_str!("dto/rpc.rs"), "pub enum RootCapabilityCommand")
    }

    fn durable_publish_endpoint_names() -> BTreeSet<&'static str> {
        [
            CANIC_TEMPLATE_PREPARE_ADMIN,
            CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            "canic_wasm_store_admin",
        ]
        .into_iter()
        .chain(CANIC_WASM_STORE_ROOT_UPDATE_METHODS.iter().copied())
        .collect()
    }

    fn release_candidate_manifest_blockers() -> BTreeSet<String> {
        let endpoint_blockers = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| {
                entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker
            })
            .map(|entry| format!("endpoint:{}", entry.endpoint));

        let root_command_blockers = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| {
                entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker
            })
            .map(|entry| format!("root-capability:{}", entry.variant));

        let pool_command_blockers = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .filter(|entry| {
                entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker
            })
            .map(|entry| format!("pool-admin:{}", entry.variant));

        endpoint_blockers
            .chain(root_command_blockers)
            .chain(pool_command_blockers)
            .collect()
    }

    fn enum_variant_names_from_source(
        source: &'static str,
        marker: &'static str,
    ) -> BTreeSet<&'static str> {
        let start = source.find(marker).expect("enum exists in source");
        let body_start = source[start..]
            .find('{')
            .map(|offset| start + offset + 1)
            .expect("enum has body");
        let body_end = source[body_start..]
            .find("\n}")
            .map(|offset| body_start + offset)
            .expect("enum body closes");

        source[body_start..body_end]
            .lines()
            .filter_map(enum_variant_name_from_line)
            .collect()
    }

    fn enum_variant_name_from_line(line: &'static str) -> Option<&'static str> {
        let line = line.trim();
        let first = line.as_bytes().first().copied()?;
        if !first.is_ascii_uppercase() {
            return None;
        }
        let end = line
            .find(|ch: char| ch == '(' || ch == '{' || ch == ',' || ch.is_whitespace())
            .unwrap_or(line.len());
        Some(&line[..end])
    }

    fn emitted_update_endpoint_names() -> BTreeSet<&'static str> {
        [
            include_str!("../../canic/src/macros/endpoints/root.rs"),
            include_str!("../../canic/src/macros/endpoints/shared.rs"),
            include_str!("../../canic/src/macros/endpoints/wasm_store.rs"),
            include_str!("../../canic/src/macros/endpoints/nonroot.rs"),
            include_str!("../../canic/src/macros/endpoints/icp_refill.rs"),
        ]
        .into_iter()
        .flat_map(update_endpoint_names_from_source)
        .collect()
    }

    fn update_endpoint_names_from_source(source: &'static str) -> Vec<&'static str> {
        let lines = source.lines().collect::<Vec<_>>();
        let mut names = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains("#[$crate::canic_update") {
                continue;
            }
            let Some(name) = lines
                .iter()
                .skip(index + 1)
                .take(6)
                .find_map(|candidate| endpoint_name_from_fn_line(candidate))
            else {
                panic!("canic_update endpoint attribute without following function");
            };
            names.push(name);
        }
        names
    }

    fn endpoint_name_from_fn_line(line: &'static str) -> Option<&'static str> {
        let marker = "fn ";
        let start = line.find(marker)? + marker.len();
        let rest = &line[start..];
        let end = rest.find('(')?;
        Some(&rest[..end])
    }
}
