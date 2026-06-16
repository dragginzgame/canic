//! Module: plan::tests::authority
//!
//! Responsibility: backup authority receipt application tests.
//! Does not own: request shape or topology preflight tests.
//! Boundary: authority receipts that upgrade dry-run plans for execution.

use super::*;

// Ensure authority receipts are the bridge from dry-run planning to execution.
#[test]
fn authority_receipts_upgrade_declared_plan_for_execution() {
    let mut plan = subtree_plan();
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);

    plan.apply_authority_preflight_receipts(
        PREFLIGHT_ID,
        &[control_receipt(APP, proven_root_control())],
        &[snapshot_read_receipt(APP, proven_root_read())],
        AS_OF,
    )
    .expect("apply authority receipts");

    assert_eq!(plan.targets[0].control_authority, proven_root_control());
    assert_eq!(plan.targets[0].snapshot_read_authority, proven_root_read());
    plan.validate_for_execution()
        .expect("receipts make plan executable");
}

// Ensure control authority preflight must cover every selected target.
#[test]
fn control_authority_receipts_must_cover_all_targets() {
    let mut plan = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        registry: &registry(),
        ..plan_input()
    })
    .expect("build subtree plan");

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(APP, proven_root_control())],
            AS_OF,
        )
        .expect_err("missing worker receipt rejects");

    std::assert_matches!(
        err,
        BackupPlanError::MissingControlAuthorityReceipt(canister) if canister == WORKER
    );
}

// Ensure receipts cannot prove authority for canisters outside the plan.
#[test]
fn authority_receipts_reject_unknown_targets() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(WORKER, proven_root_control())],
            AS_OF,
        )
        .expect_err("unknown target receipt rejects");

    std::assert_matches!(
        err,
        BackupPlanError::UnknownAuthorityReceiptTarget(canister) if canister == WORKER
    );
}

// Ensure receipt application does not treat declarations as execution proof.
#[test]
fn authority_receipts_reject_unproven_authority() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(
                APP,
                ControlAuthority::root_controller(AuthorityEvidence::Declared),
            )],
            AS_OF,
        )
        .expect_err("declared receipt rejects");

    std::assert_matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    );
}

// Ensure root-coordinated plans cannot be upgraded by operator-only proof.
#[test]
fn root_controller_plans_require_root_controller_receipts() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(
                APP,
                ControlAuthority::operator_controller(AuthorityEvidence::Proven),
            )],
            AS_OF,
        )
        .expect_err("operator controller does not satisfy root controller plan");

    std::assert_matches!(
        err,
        BackupPlanError::MissingRootController(canister) if canister == APP
    );
}

// Ensure standalone authority proofs also expire.
#[test]
fn rejects_expired_authority_receipt() {
    let mut plan = subtree_plan();

    let err = plan
        .apply_control_authority_receipts(
            PREFLIGHT_ID,
            &[control_receipt(APP, proven_root_control())],
            "unix:250",
        )
        .expect_err("expired authority receipt rejects");

    std::assert_matches!(
        err,
        BackupPlanError::PreflightReceiptExpired { preflight_id, expires_at, as_of }
            if preflight_id == PREFLIGHT_ID && expires_at == EXPIRES_AT && as_of == "unix:250"
    );
}
