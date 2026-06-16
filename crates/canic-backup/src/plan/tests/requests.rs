//! Module: plan::tests::requests
//!
//! Responsibility: backup preflight request shape tests.
//! Does not own: receipt validation or authority application.
//! Boundary: typed request projections generated from backup plans.

use super::*;

// Ensure control authority preflights have a stable typed request shape.
#[test]
fn builds_control_authority_preflight_request() {
    let plan = subtree_plan();
    let request = plan.control_authority_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.root_canister_id, ROOT);
    assert!(request.requires_root_controller);
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].role.as_deref(), Some("app"));
    assert_eq!(request.targets[0].declared_authority, proven_root_control());
}

// Ensure snapshot read preflights have a stable typed request shape.
#[test]
fn builds_snapshot_read_authority_preflight_request() {
    let plan = subtree_plan();
    let request = plan.snapshot_read_authority_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.root_canister_id, ROOT);
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].role.as_deref(), Some("app"));
    assert_eq!(request.targets[0].declared_authority, proven_root_read());
}

// Ensure topology preflights have a stable typed request shape.
#[test]
fn builds_topology_preflight_request() {
    let plan = subtree_plan();
    let request = plan.topology_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.selected_subtree_root.as_deref(), Some(APP));
    assert_eq!(request.selected_scope_kind, BackupScopeKind::Subtree);
    assert_eq!(
        request.topology_hash_before_quiesce,
        plan.topology_hash_before_quiesce
    );
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].parent_canister_id.as_deref(), Some(ROOT));
    assert_eq!(request.targets[0].depth, 1);
}

// Ensure quiescence preflights have a stable typed request shape.
#[test]
fn builds_quiescence_preflight_request() {
    let plan = subtree_plan();
    let request = plan.quiescence_preflight_request();

    assert_eq!(request.plan_id, "plan-001");
    assert_eq!(request.run_id, "run-001");
    assert_eq!(request.selected_subtree_root.as_deref(), Some(APP));
    assert_eq!(request.quiescence_policy, QuiescencePolicy::RootCoordinated);
    assert_eq!(request.targets.len(), 1);
    assert_eq!(request.targets[0].canister_id, APP);
    assert_eq!(request.targets[0].role.as_deref(), Some("app"));
}
