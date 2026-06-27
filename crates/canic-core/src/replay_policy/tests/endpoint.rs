//! Module: replay_policy::tests::endpoint
//!
//! Responsibility: verify endpoint replay-policy classifications for key surfaces.
//! Does not own: endpoint implementations or manifest construction.
//! Boundary: test-only checks over endpoint manifest rows.

use super::*;

#[test]
fn delegation_proof_batch_prepare_is_manifested_as_implemented() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_prepare_delegation_proof_batch")
        .expect("delegation batch prepare endpoint policy entry");

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
            command_kind: "auth.prepare_delegation_proof_batch.v1",
            requires_operation_id: true,
        }
    );
}

#[test]
fn root_delegation_renewal_config_upserts_are_snapshot_convergent() {
    for (endpoint, command_kind) in [
        (
            "canic_upsert_root_issuer_renewal_template",
            "auth.upsert_root_issuer_renewal_template.v1",
        ),
        (
            "canic_upsert_delegation_renewal_provisioner",
            "auth.upsert_delegation_renewal_provisioner.v1",
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == endpoint)
            .expect("root delegation renewal config policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::SnapshotConvergent { command_kind }
        );
    }
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
