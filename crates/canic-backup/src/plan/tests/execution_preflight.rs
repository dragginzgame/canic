//! Module: plan::tests::execution_preflight
//!
//! Responsibility: execution preflight receipt validation tests.
//! Does not own: authority-only receipt application.
//! Boundary: topology, quiescence, and bundled preflight receipts.

use super::*;

// Ensure topology and quiescence receipts gate mutating execution preflights.
#[test]
fn validates_execution_preflight_receipts() {
    let plan = subtree_plan();

    plan.validate_execution_preflight_receipts(
        &topology_receipt(&plan),
        &quiescence_receipt(&plan),
        PREFLIGHT_ID,
        AS_OF,
    )
    .expect("valid execution preflights");
}

// Ensure the full preflight bundle upgrades authority and validates execution gates.
#[test]
fn applies_execution_preflight_receipt_bundle() {
    let mut plan = subtree_plan();
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);
    let receipts = execution_preflight_receipts(&subtree_plan());

    plan.apply_execution_preflight_receipts(&receipts, AS_OF)
        .expect("apply execution preflight bundle");

    assert_eq!(plan.targets[0].control_authority, proven_root_control());
    assert_eq!(plan.targets[0].snapshot_read_authority, proven_root_read());
    plan.validate_for_execution()
        .expect("bundle makes plan executable");
}

// Ensure nullable receipt messages are explicit parts of the current evidence envelope.
#[test]
fn preflight_receipts_require_exact_current_optional_fields() {
    let receipts = execution_preflight_receipts(&subtree_plan());

    let mut values = [
        serde_json::to_value(&receipts.topology).expect("serialize topology receipt"),
        serde_json::to_value(&receipts.quiescence).expect("serialize quiescence receipt"),
        serde_json::to_value(&receipts.control_authority[0]).expect("serialize control receipt"),
        serde_json::to_value(&receipts.snapshot_read_authority[0]).expect("serialize read receipt"),
    ];
    for value in &mut values {
        value
            .as_object_mut()
            .expect("preflight receipt object")
            .remove("message");
    }

    assert!(serde_json::from_value::<TopologyPreflightReceipt>(values[0].clone()).is_err());
    assert!(serde_json::from_value::<QuiescencePreflightReceipt>(values[1].clone()).is_err());
    assert!(serde_json::from_value::<ControlAuthorityReceipt>(values[2].clone()).is_err());
    assert!(serde_json::from_value::<SnapshotReadAuthorityReceipt>(values[3].clone()).is_err());
}

// Ensure stale preflight bundles cannot authorize later mutation.
#[test]
fn rejects_expired_execution_preflight_bundle() {
    let mut plan = subtree_plan();
    let receipts = execution_preflight_receipts(&plan);

    let err = plan
        .apply_execution_preflight_receipts(&receipts, "unix:250")
        .expect_err("expired preflight bundle rejects");

    std::assert_matches!(
        err,
        BackupPlanError::PreflightReceiptExpired { preflight_id, expires_at, as_of }
            if preflight_id == PREFLIGHT_ID && expires_at == EXPIRES_AT && as_of == "unix:250"
    );
}

// Ensure receipt bundles cannot mix proofs from a different preflight run.
#[test]
fn rejects_mismatched_preflight_id_in_bundle_receipts() {
    let mut plan = subtree_plan();
    let mut receipts = execution_preflight_receipts(&plan);
    receipts.topology.preflight_id = "preflight-other".to_string();

    let err = plan
        .apply_execution_preflight_receipts(&receipts, AS_OF)
        .expect_err("mismatched preflight receipt rejects");

    std::assert_matches!(
        err,
        BackupPlanError::PreflightReceiptIdMismatch { expected, actual }
            if expected == PREFLIGHT_ID && actual == "preflight-other"
    );
}

// Ensure topology drift fails before mutation.
#[test]
fn rejects_topology_preflight_hash_drift() {
    let plan = subtree_plan();
    let mut receipt = topology_receipt(&plan);
    receipt.topology_hash_at_preflight =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

    let err = plan
        .validate_execution_preflight_receipts(
            &receipt,
            &quiescence_receipt(&plan),
            PREFLIGHT_ID,
            AS_OF,
        )
        .expect_err("topology drift rejects");

    std::assert_matches!(
        err,
        BackupPlanError::TopologyPreflightHashMismatch { expected, actual }
            if expected == plan.topology_hash_before_quiesce
                && actual == "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
}

// Ensure quiescence rejection fails before mutation.
#[test]
fn rejects_unaccepted_quiescence_preflight() {
    let plan = subtree_plan();
    let mut receipt = quiescence_receipt(&plan);
    receipt.accepted = false;

    let err = plan
        .validate_execution_preflight_receipts(
            &topology_receipt(&plan),
            &receipt,
            PREFLIGHT_ID,
            AS_OF,
        )
        .expect_err("quiescence rejection rejects");

    std::assert_matches!(err, BackupPlanError::QuiescencePreflightRejected);
}

// Ensure quiescence receipts cannot silently cover a different target set.
#[test]
fn rejects_quiescence_target_mismatch() {
    let plan = subtree_plan();
    let mut receipt = quiescence_receipt(&plan);
    receipt.targets.clear();

    let err = plan
        .validate_execution_preflight_receipts(
            &topology_receipt(&plan),
            &receipt,
            PREFLIGHT_ID,
            AS_OF,
        )
        .expect_err("quiescence target mismatch rejects");

    std::assert_matches!(err, BackupPlanError::QuiescencePreflightTargetsMismatch);
}
