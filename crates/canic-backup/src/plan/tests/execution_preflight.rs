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
