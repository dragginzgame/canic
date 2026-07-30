//! Module: replay_policy::tests::endpoint
//!
//! Responsibility: verify endpoint replay-policy classifications for key surfaces.
//! Does not own: endpoint implementations or manifest construction.
//! Boundary: test-only checks over endpoint manifest rows.

use super::*;
use crate::replay_policy::quota::ROOT_CHAIN_KEY_SIGNING_QUOTA_V1;

#[test]
fn chain_key_lazy_repair_is_manifested_as_costed_snapshot_convergent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_get_or_create_chain_key_delegation_proof")
        .expect("chain-key lazy repair endpoint policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(entry.cost_class, CostClass::RootChainKeySigning);
    assert_eq!(entry.quota_policy, Some(ROOT_CHAIN_KEY_SIGNING_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, None);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("auth.get_or_create_chain_key_delegation_proof.v1"),
        }
    );
}

#[test]
fn root_issuer_renewal_template_upsert_is_snapshot_convergent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_upsert_root_issuer_renewal_template")
        .expect("root issuer renewal template policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(entry.cost_class, CostClass::None);
    assert_eq!(entry.quota_policy, None);
    assert_eq!(entry.cycle_reserve_policy, None);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("auth.upsert_root_issuer_renewal_template.v1"),
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
            command_kind: replay_command_kind("auth.install_active_delegation_proof.v1"),
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
            command_kind: replay_command_kind("auth.prepare_delegated_token.v1"),
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
            command_kind: replay_command_kind("auth.prepare_role_attestation.v1"),
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
fn runtime_introspection_endpoints_are_manifested_as_read_only_queries() {
    for endpoint in ["canic_health", "canic_readiness", "canic_runtime_status"] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == endpoint)
            .expect("runtime introspection policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.endpoint_kind, EndpointKind::Query);
        assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
    }
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
            command_kind: replay_command_kind("management.canister_upgrade.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn root_component_creation_is_costed_and_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_create")
        .expect("root Component creation policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("management.control_plane.component_create.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn root_component_child_creation_is_costed_and_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_create")
        .expect("root Component Child creation policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("management.control_plane.component_child_create.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn root_component_child_install_is_costed_and_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_install")
        .expect("root Component Child install policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind(
                "management.control_plane.component_child_install.v1"
            ),
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn root_component_child_commit_is_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_commit")
        .expect("root Component Child commit policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("component_registry.commit_child.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    assert_eq!(entry.quota_policy, None);
    assert_eq!(entry.cycle_reserve_policy, None);
}

#[test]
fn root_component_child_directory_prepare_is_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_directory_prepare")
        .expect("root Component Child Directory policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("component_registry.prepare_child_directory.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    assert_eq!(entry.quota_policy, None);
    assert_eq!(entry.cycle_reserve_policy, None);
}

#[test]
fn root_component_child_runtime_activation_is_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_runtime_activate")
        .expect("root Component Child runtime activation policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("component_registry.activate_child_runtime.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    assert_eq!(entry.quota_policy, None);
    assert_eq!(entry.cycle_reserve_policy, None);
}

#[test]
fn root_component_child_membership_activation_is_response_idempotent() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_membership_activate")
        .expect("root Component Child membership activation policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("component_registry.activate_child_membership.v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    assert_eq!(entry.quota_policy, None);
    assert_eq!(entry.cycle_reserve_policy, None);
}

#[test]
fn root_component_child_reservation_is_response_idempotent() {
    let reservation = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_allocate")
        .expect("root Component Child reservation policy entry");
    let status = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_child_allocation_status")
        .expect("root Component Child reservation status policy entry");

    assert_eq!(
        reservation.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("component_registry.allocate_child.v1"),
        }
    );
    assert_eq!(reservation.cost_class, CostClass::None);
    assert_eq!(status.replay_policy, ReplayPolicy::QueryOrReadOnly);
    assert_eq!(status.endpoint_kind, EndpointKind::Query);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one policy inventory test keeps every monotonic subtree-removal transition aligned"
)]
fn root_component_subtree_removal_progress_is_replay_safe() {
    let begin = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_begin")
        .expect("root Component subtree-removal begin policy entry");
    let advance = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_advance")
        .expect("root Component subtree-removal advance policy entry");
    let stop_prepare = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_stop_prepare")
        .expect("root Component subtree-removal stop preparation policy entry");
    let stop = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_stop")
        .expect("root Component subtree-removal stop policy entry");
    let delete_prepare = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_delete_prepare")
        .expect("root Component subtree-removal deletion preparation policy entry");
    let delete = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_delete")
        .expect("root Component subtree-removal deletion policy entry");
    let membership_remove = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_membership_remove")
        .expect("root Component subtree membership-removal policy entry");
    let directory_synchronize = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| {
            entry.endpoint == "canic_root_component_subtree_removal_directory_synchronize"
        })
        .expect("root Component subtree Directory synchronization policy entry");
    let status = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_component_subtree_removal_status")
        .expect("root Component subtree-removal status policy entry");

    assert_eq!(
        begin.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("component_registry.begin_subtree_removal.v1"),
        }
    );
    assert_eq!(begin.cost_class, CostClass::None);
    assert_eq!(
        advance.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("component_registry.advance_subtree_removal.v1"),
        }
    );
    assert_eq!(advance.cost_class, CostClass::None);
    assert_eq!(
        stop_prepare.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("component_registry.prepare_subtree_stop.v1"),
        }
    );
    assert_eq!(stop_prepare.cost_class, CostClass::None);
    assert_eq!(
        stop.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("management.control_plane.component_subtree_stop.v1"),
        }
    );
    assert_eq!(stop.cost_class, CostClass::None);
    assert_eq!(
        delete_prepare.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("component_registry.prepare_subtree_delete.v1"),
        }
    );
    assert_eq!(delete_prepare.cost_class, CostClass::None);
    assert_eq!(
        delete.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind(
                "management.control_plane.component_subtree_delete.v1"
            ),
        }
    );
    assert_eq!(delete.cost_class, CostClass::None);
    assert_eq!(
        membership_remove.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind(
                "component_registry.remove_subtree_leaf_membership.v1"
            ),
        }
    );
    assert_eq!(membership_remove.cost_class, CostClass::None);
    assert_eq!(
        directory_synchronize.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind(
                "component_registry.synchronize_subtree_removal_directory.v1"
            ),
        }
    );
    assert_eq!(directory_synchronize.cost_class, CostClass::None);
    assert_eq!(status.replay_policy, ReplayPolicy::QueryOrReadOnly);
    assert_eq!(status.endpoint_kind, EndpointKind::Query);
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
            command_kind: replay_command_kind("icp.refill.v1"),
            requires_operation_id: true,
        }
    );
    assert_eq!(entry.cost_class, CostClass::ValueTransfer);
    assert_eq!(entry.quota_policy, Some(VALUE_TRANSFER_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(VALUE_TRANSFER_RESERVE_V1));
}
