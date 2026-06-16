//! Module: plan::tests::validation
//!
//! Responsibility: backup plan validation tests.
//! Does not own: registry builder or preflight request shape tests.
//! Boundary: structural and execution-readiness plan invariants.

use super::*;

// Ensure a normal subtree plan can prove authority before mutation.
#[test]
fn validates_subtree_plan_with_authority_preflights() {
    let plan = subtree_plan();

    plan.validate().expect("valid subtree plan");
    plan.validate_for_execution()
        .expect("executable subtree plan");
}

// Ensure normal backups cannot silently include root.
#[test]
fn rejects_root_in_normal_scope() {
    let mut plan = subtree_plan();
    plan.root_included = true;
    plan.targets.push(BackupTarget {
        canister_id: ROOT.to_string(),
        role: Some("root".to_string()),
        parent_canister_id: None,
        depth: 0,
        control_authority: proven_root_control(),
        snapshot_read_authority: proven_root_read(),
        identity_mode: IdentityMode::Fixed,
        expected_module_hash: None,
    });

    let err = plan
        .validate()
        .expect_err("root should require maintenance");

    std::assert_matches!(err, BackupPlanError::RootIncludedWithoutMaintenance);
}

// Ensure declared authority is still a valid planning/dry-run artifact.
#[test]
fn planning_allows_declared_authority() {
    let mut plan = subtree_plan();
    plan.snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);

    plan.validate().expect("declared authority can plan");
    let err = plan
        .validate_for_execution()
        .expect_err("declared authority cannot execute");

    std::assert_matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    );
}

// Ensure unknown authority can be represented for dry-run output.
#[test]
fn planning_allows_unknown_authority() {
    let mut plan = subtree_plan();
    plan.snapshot_read_authority = SnapshotReadAuthority::unknown();
    plan.targets[0].control_authority = ControlAuthority::unknown();
    plan.targets[0].snapshot_read_authority = SnapshotReadAuthority::unknown();

    plan.validate().expect("unknown authority can plan");
    let err = plan
        .validate_for_execution()
        .expect_err("unknown authority cannot execute");

    std::assert_matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    );
}

// Ensure unproven control authority fails before any execution can happen.
#[test]
fn rejects_unproven_control_authority() {
    let mut plan = subtree_plan();
    plan.targets[0].control_authority =
        ControlAuthority::root_controller(AuthorityEvidence::Declared);

    let err = plan
        .validate_for_execution()
        .expect_err("control authority should be proven");

    std::assert_matches!(
        err,
        BackupPlanError::UnprovenControlAuthority(canister) if canister == APP
    );
}

// Ensure snapshot read authority is a first-class preflight, not a late download error.
#[test]
fn rejects_unproven_snapshot_read_authority() {
    let mut plan = subtree_plan();
    plan.targets[0].snapshot_read_authority =
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared);

    let err = plan
        .validate_for_execution()
        .expect_err("snapshot read authority should be proven");

    std::assert_matches!(
        err,
        BackupPlanError::UnprovenTargetSnapshotReadAuthority(canister) if canister == APP
    );
}

// Ensure mutations cannot be planned before authority and quiescence checks.
#[test]
fn rejects_mutation_before_preflights() {
    let mut plan = subtree_plan();
    let stop = plan.phases.remove(4);
    plan.phases.insert(0, stop);
    reset_phase_order(&mut plan.phases);

    let err = plan
        .validate()
        .expect_err("mutation before preflight should reject");

    std::assert_matches!(
        err,
        BackupPlanError::MutationBeforePreflight { operation_id }
            if operation_id == "stop-app"
    );
}

// Ensure whole root-omitted deployment plans do not pretend to have a unique subtree root.
#[test]
fn rejects_root_omitted_deployment_scope_with_selected_root() {
    let mut plan = subtree_plan();
    plan.selected_scope_kind = BackupScopeKind::NonRootDeployment;

    let err = plan
        .validate()
        .expect_err("root-omitted deployment scope should not name one root");

    std::assert_matches!(err, BackupPlanError::NonRootDeploymentHasSelectedRoot);
}

// Ensure journals can rely on stable contiguous operation ordering.
#[test]
fn rejects_operation_order_mismatch() {
    let mut plan = subtree_plan();
    plan.phases[1].order = 42;

    let err = plan
        .validate()
        .expect_err("operation order mismatch should reject");

    std::assert_matches!(
        err,
        BackupPlanError::OperationOrderMismatch { operation_id, order, expected }
            if operation_id == "validate-control" && order == 42 && expected == 1
    );
}
