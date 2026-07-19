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

// Ensure nullable preflight request keys are explicit parts of each current contract.
#[test]
fn preflight_requests_require_exact_current_optional_fields() {
    let plan = subtree_plan();

    let request = plan.control_authority_preflight_request();
    for field in ["role", "parent_canister_id"] {
        let mut value =
            serde_json::to_value(&request.targets[0]).expect("serialize control target");
        value
            .as_object_mut()
            .expect("control target object")
            .remove(field);
        let err = serde_json::from_value::<ControlAuthorityPreflightTarget>(value)
            .expect_err("current control target field must be present");
        assert!(err.is_data());
    }

    let request = plan.snapshot_read_authority_preflight_request();
    for field in ["role", "parent_canister_id"] {
        let mut value = serde_json::to_value(&request.targets[0]).expect("serialize read target");
        value
            .as_object_mut()
            .expect("read target object")
            .remove(field);
        let err = serde_json::from_value::<SnapshotReadAuthorityPreflightTarget>(value)
            .expect_err("current read target field must be present");
        assert!(err.is_data());
    }

    let request = plan.topology_preflight_request();
    let mut value = serde_json::to_value(&request).expect("serialize topology request");
    value
        .as_object_mut()
        .expect("topology request object")
        .remove("selected_subtree_root");
    let err = serde_json::from_value::<TopologyPreflightRequest>(value)
        .expect_err("current selected_subtree_root field must be present");
    assert!(err.is_data());

    let mut value = serde_json::to_value(&request.targets[0]).expect("serialize topology target");
    value
        .as_object_mut()
        .expect("topology target object")
        .remove("parent_canister_id");
    let err = serde_json::from_value::<TopologyPreflightTarget>(value)
        .expect_err("current parent_canister_id field must be present");
    assert!(err.is_data());

    let request = plan.quiescence_preflight_request();
    let mut value = serde_json::to_value(&request).expect("serialize quiescence request");
    value
        .as_object_mut()
        .expect("quiescence request object")
        .remove("selected_subtree_root");
    let err = serde_json::from_value::<QuiescencePreflightRequest>(value)
        .expect_err("current selected_subtree_root field must be present");
    assert!(err.is_data());

    for field in ["role", "parent_canister_id"] {
        let mut value =
            serde_json::to_value(&request.targets[0]).expect("serialize quiescence target");
        value
            .as_object_mut()
            .expect("quiescence target object")
            .remove(field);
        let err = serde_json::from_value::<QuiescencePreflightTarget>(value)
            .expect_err("current quiescence target field must be present");
        assert!(err.is_data());
    }
}
