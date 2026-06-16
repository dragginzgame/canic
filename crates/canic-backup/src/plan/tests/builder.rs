//! Module: plan::tests::builder
//!
//! Responsibility: backup plan builder and selector tests.
//! Does not own: authority receipt or execution preflight validation.
//! Boundary: registry-backed target expansion and selector resolution.

use super::*;

// Ensure registry planning produces root-stays-running subtree phases.
#[test]
fn builds_subtree_plan_from_registry() {
    let plan = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        registry: &registry(),
        ..plan_input()
    })
    .expect("build subtree plan");

    assert_eq!(plan.selected_subtree_root.as_deref(), Some(APP));
    assert!(!plan.root_included);
    assert_eq!(
        plan.targets
            .iter()
            .map(|target| target.canister_id.as_str())
            .collect::<Vec<_>>(),
        vec![APP, WORKER]
    );
    assert!(
        plan.phases
            .iter()
            .all(|phase| phase.target_canister_id.as_deref() != Some(ROOT))
    );
    assert_operation_order(
        &plan,
        &[
            ("validate-topology", None),
            ("validate-control-authority", None),
            ("validate-snapshot-read-authority", None),
            ("validate-quiescence-policy", None),
            ("stop-renrk-eyaaa-aaaaa-aaada-cai", Some(APP)),
            ("stop-rno2w-sqaaa-aaaaa-aaacq-cai", Some(WORKER)),
            ("snapshot-renrk-eyaaa-aaaaa-aaada-cai", Some(APP)),
            ("snapshot-rno2w-sqaaa-aaaaa-aaacq-cai", Some(WORKER)),
            ("start-rno2w-sqaaa-aaaaa-aaacq-cai", Some(WORKER)),
            ("start-renrk-eyaaa-aaaaa-aaada-cai", Some(APP)),
        ],
    );
}

// Ensure root-omitted deployment scope expands every managed member while leaving root running.
#[test]
fn builds_root_omitted_deployment_plan_without_root_target() {
    let plan = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: None,
        selected_scope_kind: BackupScopeKind::NonRootDeployment,
        registry: &registry(),
        ..plan_input()
    })
    .expect("build root-omitted deployment plan");

    assert_eq!(plan.selected_subtree_root, None);
    assert!(!plan.root_included);
    assert_eq!(
        plan.targets
            .iter()
            .map(|target| target.canister_id.as_str())
            .collect::<Vec<_>>(),
        vec![APP, WORKER]
    );
}

// Ensure normal planning rejects a root subtree before generating mutating phases.
#[test]
fn builder_rejects_root_subtree_without_maintenance() {
    let err = build_backup_plan(BackupPlanBuildInput {
        selected_canister_id: Some(ROOT.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        registry: &registry(),
        ..plan_input()
    })
    .expect_err("normal root subtree should reject");

    std::assert_matches!(err, BackupPlanError::RootIncludedWithoutMaintenance);
}

// Ensure selectors can target explicit principals or unambiguous roles.
#[test]
fn resolves_principal_and_role_selectors() {
    let registry = registry();

    assert_eq!(
        resolve_backup_selector(&registry, APP).expect("resolve principal"),
        APP
    );
    assert_eq!(
        resolve_backup_selector(&registry, "app").expect("resolve role"),
        APP
    );
}

// Ensure role selectors fail closed when a role is not unique.
#[test]
fn rejects_ambiguous_role_selector() {
    let mut registry = registry();
    registry.push(RegistryEntry {
        pid: OTHER_WORKER.to_string(),
        role: Some("worker".to_string()),
        kind: Some("replica".to_string()),
        parent_pid: Some(APP.to_string()),
        module_hash: None,
    });

    let err =
        resolve_backup_selector(&registry, "worker").expect_err("ambiguous role should reject");

    std::assert_matches!(
        err,
        BackupPlanError::AmbiguousSelector { selector, matches }
            if selector == "worker" && matches == vec![WORKER.to_string(), OTHER_WORKER.to_string()]
    );
}

// Ensure selectors never silently target missing topology nodes.
#[test]
fn rejects_unknown_selector() {
    let err =
        resolve_backup_selector(&registry(), "missing-role").expect_err("missing selector rejects");

    assert!(
        matches!(err, BackupPlanError::UnknownSelector(selector) if selector == "missing-role")
    );
}
