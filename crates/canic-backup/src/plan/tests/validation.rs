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

// Ensure persisted plans contain the complete canonical operation projection.
#[test]
fn rejects_incomplete_operation_projection() {
    let mut plan = subtree_plan();
    plan.phases.pop();

    let err = plan
        .validate()
        .expect_err("incomplete operation projection should reject");

    std::assert_matches!(
        err,
        BackupPlanError::OperationCountMismatch {
            expected: 10,
            actual: 9
        }
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

// Ensure journals can rely on the canonical operation ordering.
#[test]
fn rejects_operation_order_mismatch() {
    let mut plan = subtree_plan();
    plan.phases[1].order = 42;

    let err = plan
        .validate()
        .expect_err("operation order mismatch should reject");

    std::assert_matches!(
        err,
        BackupPlanError::OperationProjectionMismatch {
            index: 1,
            field: "order"
        }
    );
}

// Ensure operation identities cannot diverge from the canonical projection.
#[test]
fn rejects_operation_id_mismatch() {
    let mut plan = subtree_plan();
    plan.phases[4].operation_id = "stop-other".to_string();

    let err = plan
        .validate()
        .expect_err("operation id mismatch should reject");

    std::assert_matches!(
        err,
        BackupPlanError::OperationProjectionMismatch {
            index: 4,
            field: "operation_id"
        }
    );
}

// Ensure operation kinds cannot diverge from the canonical projection.
#[test]
fn rejects_operation_kind_mismatch() {
    let mut plan = subtree_plan();
    plan.phases[4].kind = BackupOperationKind::CreateSnapshot;

    let err = plan
        .validate()
        .expect_err("operation kind mismatch should reject");

    std::assert_matches!(
        err,
        BackupPlanError::OperationProjectionMismatch {
            index: 4,
            field: "kind"
        }
    );
}

// Ensure operation targets cannot diverge from the canonical projection.
#[test]
fn rejects_operation_target_mismatch() {
    let mut plan = subtree_plan();
    plan.phases[4].target_canister_id = Some(WORKER.to_string());

    let err = plan
        .validate()
        .expect_err("operation target mismatch should reject");

    std::assert_matches!(
        err,
        BackupPlanError::OperationProjectionMismatch {
            index: 4,
            field: "target_canister_id"
        }
    );
}

// Ensure the persisted discovery hash has the current exact shape.
#[test]
fn rejects_invalid_topology_hash() {
    let mut plan = subtree_plan();
    plan.topology_hash_before_quiesce = "not-a-hash".to_string();

    let err = plan
        .validate()
        .expect_err("invalid topology hash should reject");

    std::assert_matches!(
        err,
        BackupPlanError::InvalidTopologyHash {
            field: "topology_hash_before_quiesce",
            value
        } if value == "not-a-hash"
    );
}

// Ensure selected target parent links cannot form a cycle.
#[test]
fn rejects_target_parent_cycle() {
    let mut plan = subtree_plan();
    plan.targets[0].parent_canister_id = Some(APP.to_string());

    let err = plan
        .validate()
        .expect_err("target parent cycle should reject");

    std::assert_matches!(
        err,
        BackupPlanError::TargetParentCycle { canister_id } if canister_id == APP
    );
}

// Ensure target depth agrees with every selected parent edge.
#[test]
fn rejects_target_depth_mismatch() {
    let mut plan = subtree_plan();
    plan.targets.push(worker_target(APP, 3));
    plan.phases = build_backup_phases(&plan.targets);

    let err = plan
        .validate()
        .expect_err("target depth mismatch should reject");

    std::assert_matches!(
        err,
        BackupPlanError::TargetDepthMismatch {
            canister_id,
            parent_canister_id,
            expected: 2,
            actual: 3
        } if canister_id == WORKER && parent_canister_id == APP
    );
}

// Ensure a selected subtree root is the root of its persisted target graph.
#[test]
fn rejects_selected_root_with_internal_parent() {
    let mut plan = subtree_plan();
    plan.targets[0].parent_canister_id = Some(WORKER.to_string());
    plan.targets[0].depth = 2;
    plan.targets.push(worker_target(ROOT, 1));
    plan.phases = build_backup_phases(&plan.targets);

    let err = plan
        .validate()
        .expect_err("selected root with internal parent should reject");

    std::assert_matches!(
        err,
        BackupPlanError::SelectedRootHasInternalParent {
            selected_root,
            parent_canister_id
        } if selected_root == APP && parent_canister_id == WORKER
    );
}

// Ensure every selected subtree member connects to its selected root.
#[test]
fn rejects_target_disconnected_from_selected_root() {
    let mut plan = subtree_plan();
    plan.targets.push(worker_target(ROOT, 1));
    plan.phases = build_backup_phases(&plan.targets);

    let err = plan
        .validate()
        .expect_err("disconnected target should reject");

    std::assert_matches!(
        err,
        BackupPlanError::TargetDisconnected {
            canister_id,
            expected_root
        } if canister_id == WORKER && expected_root == APP
    );
}

// Ensure a root-omitted deployment still belongs to its declared root graph.
#[test]
fn rejects_non_root_deployment_target_disconnected_from_root() {
    let mut plan = subtree_plan();
    plan.selected_scope_kind = BackupScopeKind::NonRootDeployment;
    plan.selected_subtree_root = None;
    plan.targets[0].parent_canister_id = Some(WORKER.to_string());

    let err = plan
        .validate()
        .expect_err("root-disconnected deployment target should reject");

    std::assert_matches!(
        err,
        BackupPlanError::TargetDisconnected {
            canister_id,
            expected_root
        } if canister_id == APP && expected_root == ROOT
    );
}

fn worker_target(parent_canister_id: &str, depth: u32) -> BackupTarget {
    BackupTarget {
        canister_id: WORKER.to_string(),
        role: Some("worker".to_string()),
        parent_canister_id: Some(parent_canister_id.to_string()),
        depth,
        control_authority: proven_root_control(),
        snapshot_read_authority: proven_root_read(),
        identity_mode: IdentityMode::Relocatable,
        expected_module_hash: None,
    }
}
