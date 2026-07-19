//! Module: plan::tests
//!
//! Responsibility: shared backup plan test fixtures.
//! Does not own: production plan construction or validation.
//! Boundary: fixtures for backup plan unit tests.

mod authority;
mod builder;
mod execution_preflight;
mod requests;
mod validation;

use super::*;
use crate::{manifest::IdentityMode, registry::RegistryEntry};

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const OTHER_WORKER: &str = "rdmx6-jaaaa-aaaaa-aaadq-cai";
const PREFLIGHT_ID: &str = "preflight-001";
const VALIDATED_AT: &str = "unix:100";
const EXPIRES_AT: &str = "unix:200";
const AS_OF: &str = "unix:150";

fn subtree_plan() -> BackupPlan {
    let targets = vec![BackupTarget {
        canister_id: APP.to_string(),
        role: Some("app".to_string()),
        parent_canister_id: Some(ROOT.to_string()),
        depth: 1,
        control_authority: proven_root_control(),
        snapshot_read_authority: proven_root_read(),
        identity_mode: IdentityMode::Relocatable,
        expected_module_hash: None,
    }];
    let phases = build_backup_phases(&targets);

    BackupPlan {
        plan_version: 1,
        plan_id: "plan-001".to_string(),
        run_id: "run-001".to_string(),
        fleet: "demo".to_string(),
        environment: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_subtree_root: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        root_included: false,
        requires_root_controller: true,
        snapshot_read_authority: proven_root_read(),
        quiescence_policy: QuiescencePolicy::RootCoordinated,
        topology_hash_before_quiesce:
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        targets,
        phases,
    }
}

fn plan_input<'a>() -> BackupPlanBuildInput<'a> {
    BackupPlanBuildInput {
        plan_id: "plan-001".to_string(),
        run_id: "run-001".to_string(),
        fleet: "demo".to_string(),
        environment: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce:
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        registry: &[],
        control_authority: proven_root_control(),
        snapshot_read_authority: proven_root_read(),
        quiescence_policy: QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    }
}

fn proven_root_control() -> ControlAuthority {
    ControlAuthority::root_controller(AuthorityEvidence::Proven)
}

fn proven_root_read() -> SnapshotReadAuthority {
    SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Proven)
}

fn control_receipt(canister_id: &str, authority: ControlAuthority) -> ControlAuthorityReceipt {
    ControlAuthorityReceipt {
        plan_id: "plan-001".to_string(),
        preflight_id: PREFLIGHT_ID.to_string(),
        target_canister_id: canister_id.to_string(),
        authority,
        proof_source: AuthorityProofSource::ManagementStatus,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn snapshot_read_receipt(
    canister_id: &str,
    authority: SnapshotReadAuthority,
) -> SnapshotReadAuthorityReceipt {
    SnapshotReadAuthorityReceipt {
        plan_id: "plan-001".to_string(),
        preflight_id: PREFLIGHT_ID.to_string(),
        target_canister_id: canister_id.to_string(),
        authority,
        proof_source: AuthorityProofSource::ManagementStatus,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn topology_receipt(plan: &BackupPlan) -> TopologyPreflightReceipt {
    TopologyPreflightReceipt {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
        topology_hash_at_preflight: plan.topology_hash_before_quiesce.clone(),
        targets: plan.topology_preflight_request().targets,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn quiescence_receipt(plan: &BackupPlan) -> QuiescencePreflightReceipt {
    QuiescencePreflightReceipt {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        quiescence_policy: plan.quiescence_policy.clone(),
        accepted: true,
        targets: plan.quiescence_preflight_request().targets,
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        message: None,
    }
}

fn execution_preflight_receipts(plan: &BackupPlan) -> BackupExecutionPreflightReceipts {
    BackupExecutionPreflightReceipts {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        validated_at: VALIDATED_AT.to_string(),
        expires_at: EXPIRES_AT.to_string(),
        topology: topology_receipt(plan),
        control_authority: vec![control_receipt(APP, proven_root_control())],
        snapshot_read_authority: vec![snapshot_read_receipt(APP, proven_root_read())],
        quiescence: quiescence_receipt(plan),
    }
}

fn registry() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            pid: ROOT.to_string(),
            role: Some("root".to_string()),
            kind: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: APP.to_string(),
            role: Some("app".to_string()),
            kind: Some("singleton".to_string()),
            parent_pid: Some(ROOT.to_string()),
            module_hash: None,
        },
        RegistryEntry {
            pid: WORKER.to_string(),
            role: Some("worker".to_string()),
            kind: Some("replica".to_string()),
            parent_pid: Some(APP.to_string()),
            module_hash: None,
        },
    ]
}

fn assert_operation_order(plan: &BackupPlan, expected: &[(&str, Option<&str>)]) {
    let actual = plan
        .phases
        .iter()
        .take(expected.len())
        .map(|phase| {
            (
                phase.operation_id.as_str(),
                phase.target_canister_id.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

// Ensure backup plans fail closed when unknown fields are present.
#[test]
fn backup_plan_unknown_field_fails_deserialize() {
    let mut value = serde_json::to_value(subtree_plan()).expect("serialize plan");
    value["unexpected_field"] = serde_json::Value::Bool(true);

    let err = serde_json::from_value::<BackupPlan>(value).expect_err("unknown field rejects");

    assert!(err.is_data());
}

#[test]
fn backup_plan_requires_and_validates_plan_version() {
    let mut value = serde_json::to_value(subtree_plan()).expect("serialize plan");
    value
        .as_object_mut()
        .expect("plan object")
        .remove("plan_version");
    let err = serde_json::from_value::<BackupPlan>(value).expect_err("plan version is required");
    assert!(err.is_data());

    let mut plan = subtree_plan();
    plan.plan_version = 2;
    let err = plan
        .validate()
        .expect_err("unsupported plan version rejects");
    std::assert_matches!(err, BackupPlanError::UnsupportedVersion(2));
}

#[test]
fn backup_plan_requires_exact_current_optional_fields() {
    let plan = subtree_plan();
    let mut value = serde_json::to_value(&plan).expect("serialize backup plan");
    value
        .as_object_mut()
        .expect("backup plan object")
        .remove("selected_subtree_root");
    let err = serde_json::from_value::<BackupPlan>(value)
        .expect_err("current selected_subtree_root field must be present");
    assert!(err.is_data());

    for field in ["role", "parent_canister_id", "expected_module_hash"] {
        let mut value = serde_json::to_value(&plan.targets[0]).expect("serialize backup target");
        value
            .as_object_mut()
            .expect("backup target object")
            .remove(field);

        let err = serde_json::from_value::<BackupTarget>(value)
            .expect_err("current backup target field must be present");
        assert!(err.is_data());
    }

    let mut value = serde_json::to_value(&plan.phases[0]).expect("serialize backup operation");
    value
        .as_object_mut()
        .expect("backup operation object")
        .remove("target_canister_id");
    let err = serde_json::from_value::<BackupOperation>(value)
        .expect_err("current target_canister_id field must be present");
    assert!(err.is_data());
}
