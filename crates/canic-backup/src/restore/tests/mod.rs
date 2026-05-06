use super::*;
use crate::manifest::{
    BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetSection,
    MemberVerificationChecks, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
    VerificationPlan,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const CHILD_TWO: &str = "r7inp-6aaaa-aaaaa-aaabq-cai";
const TARGET: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build a one-operation ready journal for command preview tests.
fn command_preview_journal(
    operation: RestoreApplyOperationKind,
    verification_kind: Option<&str>,
    verification_method: Option<&str>,
) -> RestoreApplyJournal {
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some("/tmp/canic-backup-restore".to_string()),
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::default(),
        pending_operations: 0,
        ready_operations: 1,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![RestoreApplyJournalOperation {
            sequence: 0,
            operation,
            state: RestoreApplyOperationState::Ready,
            state_updated_at: None,
            blocking_reasons: Vec::new(),
            restore_group: 1,
            phase_order: 0,
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            role: "root".to_string(),
            snapshot_id: Some("snap-root".to_string()),
            artifact_path: Some("artifacts/root".to_string()),
            verification_kind: verification_kind.map(str::to_string),
            verification_method: verification_method.map(str::to_string),
        }],
        operation_receipts: Vec::new(),
    };

    journal.validate().expect("journal should validate");
    journal
}

// Ensure command output receipts keep bounded tail output and byte counts.
#[test]
fn apply_command_output_bounds_to_tail_bytes() {
    let output = RestoreApplyCommandOutput::from_bytes(b"abcdef", 3);

    assert_eq!(output.text, "def");
    assert!(output.truncated);
    assert_eq!(output.original_bytes, 6);
}

// Build one valid manifest with a parent and child in the same restore group.
fn valid_manifest(identity_mode: IdentityMode) -> FleetBackupManifest {
    FleetBackupManifest {
        manifest_version: 1,
        backup_id: "fbk_test_001".to_string(),
        created_at: "2026-04-10T12:00:00Z".to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "v1".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            mode: ConsistencyMode::CrashConsistent,
            backup_units: vec![BackupUnit {
                unit_id: "whole-fleet".to_string(),
                kind: BackupUnitKind::WholeFleet,
                roles: vec!["root".to_string(), "app".to_string()],
                consistency_reason: None,
                dependency_closure: Vec::new(),
                topology_validation: "subtree-closed".to_string(),
                quiescence_strategy: None,
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            topology_hash: HASH.to_string(),
            members: vec![
                fleet_member("app", CHILD, Some(ROOT), identity_mode, 1),
                fleet_member("root", ROOT, None, IdentityMode::Fixed, 1),
            ],
        },
        verification: VerificationPlan {
            fleet_checks: Vec::new(),
            member_checks: Vec::new(),
        },
    }
}

// Build one manifest member for restore planning tests.
fn fleet_member(
    role: &str,
    canister_id: &str,
    parent_canister_id: Option<&str>,
    identity_mode: IdentityMode,
    restore_group: u16,
) -> FleetMember {
    FleetMember {
        role: role.to_string(),
        canister_id: canister_id.to_string(),
        parent_canister_id: parent_canister_id.map(str::to_string),
        subnet_canister_id: None,
        controller_hint: Some(ROOT.to_string()),
        identity_mode,
        restore_group,
        verification_class: "basic".to_string(),
        verification_checks: vec![VerificationCheck {
            kind: "call".to_string(),
            method: Some("canic_ready".to_string()),
            roles: Vec::new(),
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: format!("snap-{role}"),
            module_hash: Some(HASH.to_string()),
            wasm_hash: Some(HASH.to_string()),
            code_version: Some("v0.30.0".to_string()),
            artifact_path: format!("artifacts/{role}"),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
        },
    }
}

// Ensure in-place restore planning sorts parent before child.
#[test]
fn in_place_plan_orders_parent_before_child() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let ordered = plan.ordered_members();

    assert_eq!(plan.backup_id, "fbk_test_001");
    assert_eq!(plan.source_environment, "local");
    assert_eq!(plan.source_root_canister, ROOT);
    assert_eq!(plan.topology_hash, HASH);
    assert_eq!(plan.member_count, 2);
    assert_eq!(plan.identity_summary.fixed_members, 1);
    assert_eq!(plan.identity_summary.relocatable_members, 1);
    assert_eq!(plan.identity_summary.in_place_members, 2);
    assert_eq!(plan.identity_summary.mapped_members, 0);
    assert_eq!(plan.identity_summary.remapped_members, 0);
    assert!(plan.verification_summary.verification_required);
    assert!(plan.verification_summary.all_members_have_checks);
    assert!(plan.readiness_summary.ready);
    assert!(plan.readiness_summary.reasons.is_empty());
    assert_eq!(plan.verification_summary.fleet_checks, 0);
    assert_eq!(plan.verification_summary.member_check_groups, 0);
    assert_eq!(plan.verification_summary.member_checks, 2);
    assert_eq!(plan.verification_summary.members_with_checks, 2);
    assert_eq!(plan.verification_summary.total_checks, 2);
    assert_eq!(plan.ordering_summary.phase_count, 1);
    assert_eq!(plan.ordering_summary.dependency_free_members, 1);
    assert_eq!(plan.ordering_summary.in_group_parent_edges, 1);
    assert_eq!(plan.ordering_summary.cross_group_parent_edges, 0);
    assert_eq!(ordered[0].phase_order, 0);
    assert_eq!(ordered[1].phase_order, 1);
    assert_eq!(ordered[0].source_canister, ROOT);
    assert_eq!(ordered[1].source_canister, CHILD);
    assert_eq!(
        ordered[1].ordering_dependency,
        Some(RestoreOrderingDependency {
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            relationship: RestoreOrderingRelationship::ParentInSameGroup,
        })
    );
}

// Ensure cross-group parent dependencies are exposed when the parent phase is earlier.
#[test]
fn plan_reports_parent_dependency_from_earlier_group() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[0].restore_group = 2;
    manifest.fleet.members[1].restore_group = 1;

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let ordered = plan.ordered_members();

    assert_eq!(plan.phases.len(), 2);
    assert_eq!(plan.ordering_summary.phase_count, 2);
    assert_eq!(plan.ordering_summary.dependency_free_members, 1);
    assert_eq!(plan.ordering_summary.in_group_parent_edges, 0);
    assert_eq!(plan.ordering_summary.cross_group_parent_edges, 1);
    assert_eq!(ordered[0].source_canister, ROOT);
    assert_eq!(ordered[1].source_canister, CHILD);
    assert_eq!(
        ordered[1].ordering_dependency,
        Some(RestoreOrderingDependency {
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            relationship: RestoreOrderingRelationship::ParentInEarlierGroup,
        })
    );
}

// Ensure restore planning fails when groups would restore a child before its parent.
#[test]
fn plan_rejects_parent_in_later_restore_group() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[0].restore_group = 1;
    manifest.fleet.members[1].restore_group = 2;

    let err = RestorePlanner::plan(&manifest, None)
        .expect_err("parent-after-child group ordering should fail");

    assert!(matches!(
        err,
        RestorePlanError::ParentRestoreGroupAfterChild { .. }
    ));
}

// Ensure fixed identities cannot be remapped.
#[test]
fn fixed_identity_member_cannot_be_remapped() {
    let manifest = valid_manifest(IdentityMode::Fixed);
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
        ],
    };

    let err = RestorePlanner::plan(&manifest, Some(&mapping))
        .expect_err("fixed member remap should fail");

    assert!(matches!(err, RestorePlanError::FixedIdentityRemap { .. }));
}

// Ensure relocatable identities may be mapped when all members are covered.
#[test]
fn relocatable_member_can_be_mapped() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
        ],
    };

    let plan = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");
    let child = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.source_canister == CHILD)
        .expect("child member should be planned");

    assert_eq!(plan.identity_summary.fixed_members, 1);
    assert_eq!(plan.identity_summary.relocatable_members, 1);
    assert_eq!(plan.identity_summary.in_place_members, 1);
    assert_eq!(plan.identity_summary.mapped_members, 2);
    assert_eq!(plan.identity_summary.remapped_members, 1);
    assert_eq!(child.target_canister, TARGET);
    assert_eq!(child.parent_target_canister, Some(ROOT.to_string()));
}

// Ensure restore plans carry enough metadata for operator preflight.
#[test]
fn plan_members_include_snapshot_and_verification_metadata() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let root = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.source_canister == ROOT)
        .expect("root member should be planned");

    assert_eq!(root.identity_mode, IdentityMode::Fixed);
    assert_eq!(root.verification_class, "basic");
    assert_eq!(root.verification_checks[0].kind, "call");
    assert_eq!(root.source_snapshot.snapshot_id, "snap-root");
    assert_eq!(root.source_snapshot.artifact_path, "artifacts/root");
}

// Ensure restore plans make mapping mode explicit.
#[test]
fn plan_includes_mapping_summary() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let in_place = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert!(!in_place.identity_summary.mapping_supplied);
    assert!(!in_place.identity_summary.all_sources_mapped);
    assert_eq!(in_place.identity_summary.mapped_members, 0);

    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
        ],
    };
    let mapped = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");

    assert!(mapped.identity_summary.mapping_supplied);
    assert!(mapped.identity_summary.all_sources_mapped);
    assert_eq!(mapped.identity_summary.mapped_members, 2);
    assert_eq!(mapped.identity_summary.remapped_members, 1);
}

// Ensure restore plans summarize snapshot provenance completeness.
#[test]
fn plan_includes_snapshot_summary() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[1].source_snapshot.module_hash = None;
    manifest.fleet.members[1].source_snapshot.wasm_hash = None;
    manifest.fleet.members[1].source_snapshot.checksum = None;

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert!(!plan.snapshot_summary.all_members_have_module_hash);
    assert!(!plan.snapshot_summary.all_members_have_wasm_hash);
    assert!(plan.snapshot_summary.all_members_have_code_version);
    assert!(!plan.snapshot_summary.all_members_have_checksum);
    assert_eq!(plan.snapshot_summary.members_with_module_hash, 1);
    assert_eq!(plan.snapshot_summary.members_with_wasm_hash, 1);
    assert_eq!(plan.snapshot_summary.members_with_code_version, 2);
    assert_eq!(plan.snapshot_summary.members_with_checksum, 1);
    assert!(!plan.readiness_summary.ready);
    assert_eq!(
        plan.readiness_summary.reasons,
        [
            "missing-module-hash",
            "missing-wasm-hash",
            "missing-snapshot-checksum"
        ]
    );
}

// Ensure restore plans summarize manifest-level verification work.
#[test]
fn plan_includes_verification_summary() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.verification.fleet_checks.push(VerificationCheck {
        kind: "fleet-ready".to_string(),
        method: None,
        roles: Vec::new(),
    });
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "app".to_string(),
            checks: vec![VerificationCheck {
                kind: "app-ready".to_string(),
                method: Some("ready".to_string()),
                roles: Vec::new(),
            }],
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert!(plan.verification_summary.verification_required);
    assert!(plan.verification_summary.all_members_have_checks);
    let app = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.role == "app")
        .expect("app member should be planned");
    assert_eq!(app.verification_checks.len(), 2);
    assert_eq!(plan.fleet_verification_checks.len(), 1);
    assert_eq!(plan.fleet_verification_checks[0].kind, "fleet-ready");
    assert_eq!(plan.verification_summary.fleet_checks, 1);
    assert_eq!(plan.verification_summary.member_check_groups, 1);
    assert_eq!(plan.verification_summary.member_checks, 3);
    assert_eq!(plan.verification_summary.members_with_checks, 2);
    assert_eq!(plan.verification_summary.total_checks, 4);
}

// Ensure restore plans summarize the concrete operation counts automation will schedule.
#[test]
fn plan_includes_operation_summary() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert_eq!(plan.operation_summary.planned_snapshot_uploads, 2);
    assert_eq!(plan.operation_summary.planned_snapshot_loads, 2);
    assert_eq!(plan.operation_summary.planned_code_reinstalls, 0);
    assert_eq!(plan.operation_summary.planned_verification_checks, 2);
    assert_eq!(plan.operation_summary.planned_operations, 6);
    assert_eq!(plan.operation_summary.planned_phases, 1);
}

// Ensure restore plans carry manifest design conformance for smoke checks.
#[test]
fn plan_includes_design_conformance_report() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let design = plan
        .design_conformance
        .as_ref()
        .expect("new plans should carry design conformance");

    assert!(design.design_v1_ready);
    assert!(design.topology.design_v1_ready);
    assert!(design.backup_units.design_v1_ready);
    assert!(design.quiescence.design_v1_ready);
    assert!(design.verification.design_v1_ready);
    assert!(design.snapshot_provenance.design_v1_ready);
    assert!(design.restore_order.design_v1_ready);
}

// Ensure older restore plan JSON remains readable after adding newer fields.
#[test]
fn restore_plan_defaults_missing_newer_restore_fields() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let mut value = serde_json::to_value(&plan).expect("serialize plan");
    value
        .as_object_mut()
        .expect("plan should serialize as an object")
        .remove("fleet_verification_checks");
    value
        .as_object_mut()
        .expect("plan should serialize as an object")
        .remove("design_conformance");
    let operation_summary = value
        .get_mut("operation_summary")
        .and_then(serde_json::Value::as_object_mut)
        .expect("operation summary should serialize as an object");
    operation_summary.remove("planned_snapshot_uploads");
    operation_summary.remove("planned_operations");

    let decoded: RestorePlan = serde_json::from_value(value).expect("decode old plan shape");
    let status = RestoreStatus::from_plan(&decoded);
    let dry_run =
        RestoreApplyDryRun::try_from_plan(&decoded, None).expect("old plan should dry-run");

    assert!(decoded.fleet_verification_checks.is_empty());
    assert_eq!(decoded.design_conformance, None);
    assert_eq!(decoded.operation_summary.planned_snapshot_uploads, 0);
    assert_eq!(decoded.operation_summary.planned_operations, 0);
    assert_eq!(status.planned_snapshot_uploads, 2);
    assert_eq!(status.planned_operations, 6);
    assert_eq!(dry_run.planned_snapshot_uploads, 2);
    assert_eq!(dry_run.planned_operations, 6);
    assert_eq!(decoded.backup_id, plan.backup_id);
    assert_eq!(decoded.member_count, plan.member_count);
}

// Ensure initial restore status mirrors the no-mutation restore plan.
#[test]
fn restore_status_starts_all_members_as_planned() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let status = RestoreStatus::from_plan(&plan);

    assert_eq!(status.status_version, 1);
    assert_eq!(status.backup_id.as_str(), plan.backup_id.as_str());
    assert_eq!(
        status.source_environment.as_str(),
        plan.source_environment.as_str()
    );
    assert_eq!(
        status.source_root_canister.as_str(),
        plan.source_root_canister.as_str()
    );
    assert_eq!(status.topology_hash.as_str(), plan.topology_hash.as_str());
    assert!(status.ready);
    assert!(status.readiness_reasons.is_empty());
    assert!(status.verification_required);
    assert_eq!(status.member_count, 2);
    assert_eq!(status.phase_count, 1);
    assert_eq!(status.planned_snapshot_uploads, 2);
    assert_eq!(status.planned_snapshot_loads, 2);
    assert_eq!(status.planned_code_reinstalls, 0);
    assert_eq!(status.planned_verification_checks, 2);
    assert_eq!(status.planned_operations, 6);
    assert_eq!(status.phases.len(), 1);
    assert_eq!(status.phases[0].restore_group, 1);
    assert_eq!(status.phases[0].members.len(), 2);
    assert_eq!(
        status.phases[0].members[0].state,
        RestoreMemberState::Planned
    );
    assert_eq!(status.phases[0].members[0].source_canister, ROOT);
    assert_eq!(status.phases[0].members[0].target_canister, ROOT);
    assert_eq!(status.phases[0].members[0].snapshot_id, "snap-root");
    assert_eq!(status.phases[0].members[0].artifact_path, "artifacts/root");
    assert_eq!(
        status.phases[0].members[1].state,
        RestoreMemberState::Planned
    );
    assert_eq!(status.phases[0].members[1].source_canister, CHILD);
}

// Ensure apply dry-runs render ordered operations without mutating targets.
#[test]
fn apply_dry_run_renders_ordered_member_operations() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let status = RestoreStatus::from_plan(&plan);
    let dry_run =
        RestoreApplyDryRun::try_from_plan(&plan, Some(&status)).expect("dry-run should build");

    assert_eq!(dry_run.dry_run_version, 1);
    assert_eq!(dry_run.backup_id.as_str(), "fbk_test_001");
    assert!(dry_run.ready);
    assert!(dry_run.status_supplied);
    assert_eq!(dry_run.member_count, 2);
    assert_eq!(dry_run.phase_count, 1);
    assert_eq!(dry_run.planned_snapshot_uploads, 2);
    assert_eq!(dry_run.planned_snapshot_loads, 2);
    assert_eq!(dry_run.planned_code_reinstalls, 0);
    assert_eq!(dry_run.planned_verification_checks, 2);
    assert_eq!(dry_run.planned_operations, 6);
    assert_eq!(dry_run.rendered_operations, 6);
    assert_eq!(dry_run.operation_counts.snapshot_uploads, 2);
    assert_eq!(dry_run.operation_counts.snapshot_loads, 2);
    assert_eq!(dry_run.operation_counts.code_reinstalls, 0);
    assert_eq!(dry_run.operation_counts.member_verifications, 2);
    assert_eq!(dry_run.operation_counts.fleet_verifications, 0);
    assert_eq!(dry_run.operation_counts.verification_operations, 2);
    assert_eq!(dry_run.phases.len(), 1);

    let operations = &dry_run.phases[0].operations;
    assert_eq!(operations[0].sequence, 0);
    assert_eq!(
        operations[0].operation,
        RestoreApplyOperationKind::UploadSnapshot
    );
    assert_eq!(operations[0].source_canister, ROOT);
    assert_eq!(operations[0].target_canister, ROOT);
    assert_eq!(operations[0].snapshot_id, Some("snap-root".to_string()));
    assert_eq!(
        operations[0].artifact_path,
        Some("artifacts/root".to_string())
    );
    assert_eq!(
        operations[1].operation,
        RestoreApplyOperationKind::LoadSnapshot
    );
    assert_eq!(
        operations[2].operation,
        RestoreApplyOperationKind::VerifyMember
    );
    assert_eq!(operations[2].verification_kind, Some("call".to_string()));
    assert_eq!(
        operations[2].verification_method,
        Some("canic_ready".to_string())
    );
    assert!(
        !operations
            .iter()
            .any(|operation| operation.operation == RestoreApplyOperationKind::ReinstallCode)
    );
    assert_eq!(operations[3].source_canister, CHILD);
    assert_eq!(
        operations[5].operation,
        RestoreApplyOperationKind::VerifyMember
    );
}

// Ensure apply dry-runs append fleet verification after member operations.
#[test]
fn apply_dry_run_renders_fleet_verification_operations() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.verification.fleet_checks.push(VerificationCheck {
        kind: "fleet-ready".to_string(),
        method: Some("canic_fleet_ready".to_string()),
        roles: Vec::new(),
    });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");

    assert_eq!(plan.operation_summary.planned_verification_checks, 3);
    assert_eq!(dry_run.rendered_operations, 7);
    let operation = dry_run.phases[0]
        .operations
        .last()
        .expect("fleet verification operation should be rendered");
    assert_eq!(operation.sequence, 6);
    assert_eq!(operation.operation, RestoreApplyOperationKind::VerifyFleet);
    assert_eq!(operation.source_canister, ROOT);
    assert_eq!(operation.target_canister, ROOT);
    assert_eq!(operation.role, "fleet");
    assert_eq!(operation.snapshot_id, None);
    assert_eq!(operation.artifact_path, None);
    assert_eq!(operation.verification_kind, Some("fleet-ready".to_string()));
    assert_eq!(
        operation.verification_method,
        Some("canic_fleet_ready".to_string())
    );
}

// Ensure apply dry-run operation sequences remain unique across phases.
#[test]
fn apply_dry_run_sequences_operations_across_phases() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[0].restore_group = 2;

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");

    assert_eq!(dry_run.phases.len(), 2);
    assert_eq!(dry_run.rendered_operations, 6);
    assert_eq!(dry_run.phases[0].operations[0].sequence, 0);
    assert_eq!(dry_run.phases[0].operations[2].sequence, 2);
    assert_eq!(dry_run.phases[1].operations[0].sequence, 3);
    assert_eq!(dry_run.phases[1].operations[2].sequence, 5);
}

// Ensure apply dry-runs can prove referenced artifacts exist and match checksums.
#[test]
fn apply_dry_run_validates_artifacts_under_backup_root() {
    let root = temp_dir("canic-restore-apply-artifacts-ok");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");

    let validation = dry_run
        .artifact_validation
        .expect("artifact validation should be present");
    assert_eq!(validation.checked_members, 2);
    assert!(validation.artifacts_present);
    assert!(validation.checksums_verified);
    assert_eq!(validation.members_with_expected_checksums, 2);
    assert_eq!(validation.checks[0].source_canister, ROOT);
    assert!(validation.checks[0].checksum_verified);

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure an artifact-validated apply dry-run produces a ready initial journal.
#[test]
fn apply_journal_marks_validated_operations_ready() {
    let root = temp_dir("canic-restore-apply-journal-ready");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(journal.journal_version, 1);
    assert_eq!(journal.backup_id.as_str(), "fbk_test_001");
    assert!(journal.ready);
    assert!(journal.blocked_reasons.is_empty());
    assert_eq!(journal.operation_count, 6);
    assert_eq!(journal.ready_operations, 6);
    assert_eq!(journal.blocked_operations, 0);
    assert_eq!(journal.operations[0].sequence, 0);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert!(journal.operations[0].blocking_reasons.is_empty());
}

// Ensure apply journals block when artifact validation was not supplied.
#[test]
fn apply_journal_blocks_without_artifact_validation() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);

    assert!(!journal.ready);
    assert_eq!(journal.ready_operations, 0);
    assert_eq!(journal.blocked_operations, 6);
    assert!(
        journal
            .blocked_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
    assert!(
        journal.operations[0]
            .blocking_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
}

// Ensure apply journal status exposes compact readiness and next-operation state.
#[test]
fn apply_journal_status_reports_next_ready_operation() {
    let root = temp_dir("canic-restore-apply-journal-status");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let status = journal.status();
    let report = journal.report();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(status.status_version, 1);
    assert_eq!(status.backup_id.as_str(), "fbk_test_001");
    assert!(status.ready);
    assert!(!status.complete);
    assert_eq!(status.operation_count, 6);
    assert_eq!(status.operation_counts.snapshot_uploads, 2);
    assert_eq!(status.operation_counts.snapshot_loads, 2);
    assert_eq!(status.operation_counts.code_reinstalls, 0);
    assert_eq!(status.operation_counts.member_verifications, 2);
    assert_eq!(status.operation_counts.fleet_verifications, 0);
    assert_eq!(status.operation_counts.verification_operations, 2);
    assert!(status.operation_counts_supplied);
    assert_eq!(journal.operation_counts, status.operation_counts);
    assert_eq!(report.operation_counts, status.operation_counts);
    assert!(report.operation_counts_supplied);
    assert_eq!(status.progress.operation_count, 6);
    assert_eq!(status.progress.completed_operations, 0);
    assert_eq!(status.progress.remaining_operations, 6);
    assert_eq!(status.progress.transitionable_operations, 6);
    assert_eq!(status.progress.attention_operations, 0);
    assert_eq!(status.progress.completion_basis_points, 0);
    assert_eq!(report.progress, status.progress);
    assert_eq!(status.pending_summary.pending_operations, 0);
    assert!(!status.pending_summary.pending_operation_available);
    assert_eq!(status.pending_summary.pending_sequence, None);
    assert_eq!(status.pending_summary.pending_operation, None);
    assert_eq!(status.pending_summary.pending_updated_at, None);
    assert!(!status.pending_summary.pending_updated_at_known);
    assert_eq!(report.pending_summary, status.pending_summary);
    assert_eq!(status.ready_operations, 6);
    assert_eq!(status.next_ready_sequence, Some(0));
    assert_eq!(
        status.next_ready_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
    assert_eq!(status.next_transition_sequence, Some(0));
    assert_eq!(
        status.next_transition_state,
        Some(RestoreApplyOperationState::Ready)
    );
    assert_eq!(
        status.next_transition_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
}

// Ensure next-operation output exposes the full next ready journal row.
#[test]
fn apply_journal_next_operation_reports_full_ready_row() {
    let root = temp_dir("canic-restore-apply-journal-next");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed(0)
        .expect("mark operation completed");
    let next = journal.next_operation();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(next.ready);
    assert!(!next.complete);
    assert!(next.operation_available);
    let operation = next.operation.expect("next operation");
    assert_eq!(operation.sequence, 1);
    assert_eq!(operation.state, RestoreApplyOperationState::Ready);
    assert_eq!(operation.operation, RestoreApplyOperationKind::LoadSnapshot);
    assert_eq!(operation.source_canister, ROOT);
}

// Ensure blocked journals report no next ready operation.
#[test]
fn apply_journal_next_operation_reports_blocked_state() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let next = journal.next_operation();

    assert!(!next.ready);
    assert!(!next.operation_available);
    assert!(next.operation.is_none());
    assert!(
        next.blocked_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
}

// Ensure command previews expose the dfx upload command without executing it.
#[test]
fn apply_journal_command_preview_reports_upload_command() {
    let root = temp_dir("canic-restore-apply-command-upload");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview();
    let expected_artifact_path = root.join("artifacts/root").to_string_lossy().to_string();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(preview.ready);
    assert!(preview.operation_available);
    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(command.program, "dfx");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            "--dir".to_string(),
            expected_artifact_path,
            ROOT.to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure command previews carry configured dfx program and network.
#[test]
fn apply_journal_command_preview_honors_command_config() {
    let root = temp_dir("canic-restore-apply-command-config");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
        program: "/tmp/dfx".to_string(),
        network: Some("local".to_string()),
    });
    let expected_artifact_path = root.join("artifacts/root").to_string_lossy().to_string();

    fs::remove_dir_all(root).expect("remove temp root");
    let command = preview.command.expect("command preview");
    assert_eq!(command.program, "/tmp/dfx");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "--network".to_string(),
            "local".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            "--dir".to_string(),
            expected_artifact_path,
            ROOT.to_string(),
        ]
    );
}

// Ensure command previews expose stopped-canister hints for snapshot load.
#[test]
fn apply_journal_command_preview_reports_load_command() {
    let root = temp_dir("canic-restore-apply-command-load");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed(0)
        .expect("mark upload completed");
    journal
        .record_operation_receipt(RestoreApplyOperationReceipt::completed_upload(
            &journal.operations[0],
            "target-snap-root".to_string(),
        ))
        .expect("record upload receipt");
    let preview = journal.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "load".to_string(),
            ROOT.to_string(),
            "target-snap-root".to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(command.requires_stopped_canister);
}

// Ensure load commands cannot render until upload receipts provide target IDs.
#[test]
fn apply_journal_load_command_requires_uploaded_snapshot_receipt() {
    let root = temp_dir("canic-restore-apply-command-load-receipt");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed(0)
        .expect("mark upload completed");
    let preview = journal.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(preview.operation_available);
    assert!(!preview.command_available);
    assert_eq!(
        preview
            .operation
            .expect("next operation should be load")
            .operation,
        RestoreApplyOperationKind::LoadSnapshot
    );
}

// Ensure command previews expose reinstall commands without executing them.
#[test]
fn apply_journal_command_preview_reports_reinstall_command() {
    let journal = command_preview_journal(RestoreApplyOperationKind::ReinstallCode, None, None);
    let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
        program: "dfx".to_string(),
        network: Some("local".to_string()),
    });

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "--network".to_string(),
            "local".to_string(),
            "install".to_string(),
            "--mode".to_string(),
            "reinstall".to_string(),
            "--yes".to_string(),
            ROOT.to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure status verification previews use `dfx canister status`.
#[test]
fn apply_journal_command_preview_reports_status_verification_command() {
    let journal = command_preview_journal(
        RestoreApplyOperationKind::VerifyMember,
        Some("status"),
        None,
    );
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "status".to_string(),
            ROOT.to_string()
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure method verification previews use `dfx canister call`.
#[test]
fn apply_journal_command_preview_reports_method_verification_command() {
    let journal = command_preview_journal(
        RestoreApplyOperationKind::VerifyMember,
        Some("query"),
        Some("health"),
    );
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "call".to_string(),
            "--query".to_string(),
            ROOT.to_string(),
            "health".to_string(),
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure fleet verification previews call the declared method on the target root.
#[test]
fn apply_journal_command_preview_reports_fleet_verification_command() {
    let journal = command_preview_journal(
        RestoreApplyOperationKind::VerifyFleet,
        Some("fleet-ready"),
        Some("canic_fleet_ready"),
    );
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "call".to_string(),
            "--query".to_string(),
            ROOT.to_string(),
            "canic_fleet_ready".to_string(),
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
    assert_eq!(
        command.note,
        "runs the declared fleet verification method as a query call"
    );
}

// Ensure method verification rows must carry the method they will call.
#[test]
fn apply_journal_validation_rejects_method_verification_without_method() {
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: None,
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::default(),
        pending_operations: 0,
        ready_operations: 1,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![RestoreApplyJournalOperation {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            state: RestoreApplyOperationState::Ready,
            state_updated_at: None,
            blocking_reasons: Vec::new(),
            restore_group: 1,
            phase_order: 0,
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            role: "root".to_string(),
            snapshot_id: Some("snap-root".to_string()),
            artifact_path: Some("artifacts/root".to_string()),
            verification_kind: Some("query".to_string()),
            verification_method: None,
        }],
        operation_receipts: Vec::new(),
    };

    let err = journal
        .validate()
        .expect_err("method verification without method should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            field: "operations[].verification_method",
        }
    ));
}

// Ensure apply journal validation rejects inconsistent state counts.
#[test]
fn apply_journal_validation_rejects_count_mismatch() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal.blocked_operations = 0;

    let err = journal.validate().expect_err("count mismatch should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::CountMismatch {
            field: "blocked_operations",
            ..
        }
    ));
}

// Ensure supplied operation-kind counts must match concrete journal rows.
#[test]
fn apply_journal_validation_rejects_operation_kind_count_mismatch() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);
    journal.operation_counts = RestoreApplyOperationKindCounts {
        snapshot_uploads: 0,
        snapshot_loads: 1,
        code_reinstalls: 0,
        member_verifications: 0,
        fleet_verifications: 0,
        verification_operations: 0,
    };

    let err = journal
        .validate()
        .expect_err("operation-kind count mismatch should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::CountMismatch {
            field: "operation_counts.snapshot_uploads",
            reported: 0,
            actual: 1,
        }
    ));
}

// Ensure older journals without operation-kind counts still validate.
#[test]
fn apply_journal_defaults_missing_operation_kind_counts() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);
    journal.operation_counts =
        RestoreApplyOperationKindCounts::from_operations(&journal.operations);
    let mut value = serde_json::to_value(&journal).expect("serialize journal");
    value
        .as_object_mut()
        .expect("journal should serialize as an object")
        .remove("operation_counts");

    let decoded: RestoreApplyJournal =
        serde_json::from_value(value).expect("decode old journal shape");
    decoded.validate().expect("old journal should validate");
    let status = decoded.status();

    assert_eq!(
        decoded.operation_counts,
        RestoreApplyOperationKindCounts::default()
    );
    assert_eq!(status.operation_counts.snapshot_uploads, 1);
    assert_eq!(status.operation_counts.snapshot_loads, 0);
    assert!(!status.operation_counts_supplied);
}

// Ensure apply journal validation rejects duplicate operation sequences.
#[test]
fn apply_journal_validation_rejects_duplicate_sequences() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal.operations[1].sequence = journal.operations[0].sequence;

    let err = journal
        .validate()
        .expect_err("duplicate sequence should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::DuplicateSequence(0)
    ));
}

// Ensure failed journal operations must explain why execution failed.
#[test]
fn apply_journal_validation_rejects_failed_without_reason() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal.operations[0].state = RestoreApplyOperationState::Failed;
    journal.operations[0].blocking_reasons = Vec::new();
    journal.blocked_operations -= 1;
    journal.failed_operations = 1;

    let err = journal
        .validate()
        .expect_err("failed operation without reason should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::FailureReasonRequired(0)
    ));
}

// Ensure claiming a ready operation marks it pending and keeps it resumable.
#[test]
fn apply_journal_mark_next_operation_pending_claims_first_operation() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    journal
        .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
        .expect("mark operation pending");
    let status = journal.status();
    let report = journal.report();
    let next = journal.next_operation();
    let preview = journal.next_command_preview();

    assert_eq!(journal.pending_operations, 1);
    assert_eq!(journal.ready_operations, 0);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Pending
    );
    assert_eq!(
        journal.operations[0].state_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert_eq!(status.next_ready_sequence, None);
    assert_eq!(status.next_transition_sequence, Some(0));
    assert_eq!(
        status.next_transition_state,
        Some(RestoreApplyOperationState::Pending)
    );
    assert_eq!(
        status.next_transition_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert_eq!(status.pending_summary.pending_operations, 1);
    assert!(status.pending_summary.pending_operation_available);
    assert_eq!(status.pending_summary.pending_sequence, Some(0));
    assert_eq!(
        status.pending_summary.pending_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
    assert_eq!(
        status.pending_summary.pending_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert!(status.pending_summary.pending_updated_at_known);
    assert_eq!(report.pending_summary, status.pending_summary);
    assert!(next.operation_available);
    assert_eq!(
        next.operation.expect("next operation").state,
        RestoreApplyOperationState::Pending
    );
    assert!(preview.operation_available);
    assert!(preview.command_available);
    assert_eq!(
        preview.operation.expect("preview operation").state,
        RestoreApplyOperationState::Pending
    );
}

// Ensure a pending claim can be released back to ready for retry.
#[test]
fn apply_journal_mark_next_operation_ready_unclaims_pending_operation() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    journal
        .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
        .expect("mark operation pending");
    journal
        .mark_next_operation_ready_at(Some("2026-05-04T12:01:00Z".to_string()))
        .expect("mark operation ready");
    let status = journal.status();
    let next = journal.next_operation();

    assert_eq!(journal.pending_operations, 0);
    assert_eq!(journal.ready_operations, 1);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert_eq!(
        journal.operations[0].state_updated_at.as_deref(),
        Some("2026-05-04T12:01:00Z")
    );
    assert_eq!(status.next_ready_sequence, Some(0));
    assert_eq!(status.next_transition_sequence, Some(0));
    assert_eq!(
        status.next_transition_state,
        Some(RestoreApplyOperationState::Ready)
    );
    assert_eq!(
        status.next_transition_updated_at.as_deref(),
        Some("2026-05-04T12:01:00Z")
    );
    assert_eq!(
        next.operation.expect("next operation").state,
        RestoreApplyOperationState::Ready
    );
}

// Ensure empty state update markers are rejected during journal validation.
#[test]
fn apply_journal_validation_rejects_empty_state_updated_at() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    journal.operations[0].state_updated_at = Some(String::new());
    let err = journal
        .validate()
        .expect_err("empty state update marker should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::MissingField("operations[].state_updated_at")
    ));
}

// Ensure operation-specific fields are required before command rendering.
#[test]
fn apply_journal_validation_rejects_missing_operation_fields() {
    let mut upload = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);
    upload.operations[0].artifact_path = None;
    let err = upload
        .validate()
        .expect_err("upload without artifact path should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::UploadSnapshot,
            field: "operations[].artifact_path",
        }
    ));

    let mut load = command_preview_journal(RestoreApplyOperationKind::LoadSnapshot, None, None);
    load.operations[0].snapshot_id = None;
    let err = load
        .validate()
        .expect_err("load without snapshot id should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::LoadSnapshot,
            field: "operations[].snapshot_id",
        }
    ));

    let mut verify = command_preview_journal(
        RestoreApplyOperationKind::VerifyMember,
        Some("query"),
        Some("health"),
    );
    verify.operations[0].verification_method = None;
    let err = verify
        .validate()
        .expect_err("method verification without method should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            field: "operations[].verification_method",
        }
    ));
}

// Ensure unclaim fails when the next transitionable operation is not pending.
#[test]
fn apply_journal_mark_next_operation_ready_rejects_without_pending_operation() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    let err = journal
        .mark_next_operation_ready()
        .expect_err("ready operation should not unclaim");

    assert!(matches!(err, RestoreApplyJournalError::NoPendingOperation));
    assert_eq!(journal.ready_operations, 1);
    assert_eq!(journal.pending_operations, 0);
}

// Ensure pending claims cannot skip earlier ready operations.
#[test]
fn apply_journal_mark_pending_rejects_out_of_order_operation() {
    let root = temp_dir("canic-restore-apply-journal-pending-out-of-order");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_pending(1)
        .expect_err("out-of-order pending claim should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OutOfOrderOperationTransition {
            requested: 1,
            next: 0
        }
    ));
    assert_eq!(journal.pending_operations, 0);
    assert_eq!(journal.ready_operations, 6);
}

// Ensure completing a journal operation updates counts and advances status.
#[test]
fn apply_journal_mark_completed_advances_next_ready_operation() {
    let root = temp_dir("canic-restore-apply-journal-completed");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal
        .mark_operation_completed(0)
        .expect("mark operation completed");
    let status = journal.status();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(journal.completed_operations, 1);
    assert_eq!(journal.ready_operations, 5);
    assert_eq!(status.next_ready_sequence, Some(1));
    assert_eq!(status.progress.completed_operations, 1);
    assert_eq!(status.progress.remaining_operations, 5);
    assert_eq!(status.progress.transitionable_operations, 5);
    assert_eq!(status.progress.attention_operations, 0);
    assert_eq!(status.progress.completion_basis_points, 1666);
}

// Ensure journal transitions cannot skip earlier ready operations.
#[test]
fn apply_journal_mark_completed_rejects_out_of_order_operation() {
    let root = temp_dir("canic-restore-apply-journal-out-of-order");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_completed(1)
        .expect_err("out-of-order operation should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OutOfOrderOperationTransition {
            requested: 1,
            next: 0
        }
    ));
    assert_eq!(journal.completed_operations, 0);
    assert_eq!(journal.ready_operations, 6);
}

// Ensure failed journal operations carry a reason and update counts.
#[test]
fn apply_journal_mark_failed_records_reason() {
    let root = temp_dir("canic-restore-apply-journal-failed");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal
        .mark_operation_failed(0, "dfx-load-failed".to_string())
        .expect("mark operation failed");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Failed
    );
    assert_eq!(
        journal.operations[0].blocking_reasons,
        vec!["dfx-load-failed".to_string()]
    );
    assert_eq!(journal.failed_operations, 1);
    assert_eq!(journal.ready_operations, 5);
}

// Ensure failed operations can move back to ready for a retry.
#[test]
fn apply_journal_retry_failed_operation_marks_ready() {
    let root = temp_dir("canic-restore-apply-journal-retry-failed");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_failed(0, "dfx-upload-failed".to_string())
        .expect("mark failed operation");
    journal
        .retry_failed_operation_at(0, Some("2026-05-04T12:03:00Z".to_string()))
        .expect("retry failed operation");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(journal.failed_operations, 0);
    assert_eq!(journal.ready_operations, 6);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert!(journal.operations[0].blocking_reasons.is_empty());
}

// Ensure blocked operations cannot be manually completed before blockers clear.
#[test]
fn apply_journal_rejects_blocked_operation_completion() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_completed(0)
        .expect_err("blocked operation should not complete");

    assert!(matches!(
        err,
        RestoreApplyJournalError::InvalidOperationTransition { sequence: 0, .. }
    ));
}

// Ensure apply dry-runs fail closed when a referenced artifact is missing.
#[test]
fn apply_dry_run_rejects_missing_artifacts() {
    let root = temp_dir("canic-restore-apply-artifacts-missing");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[0].source_snapshot.artifact_path = "missing-child".to_string();

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let err = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect_err("missing artifact should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreApplyDryRunError::ArtifactMissing { .. }
    ));
}

// Ensure apply dry-runs reject artifact paths that escape the backup directory.
#[test]
fn apply_dry_run_rejects_artifact_path_traversal() {
    let root = temp_dir("canic-restore-apply-artifacts-traversal");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[1].source_snapshot.artifact_path = "../outside".to_string();

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let err = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect_err("path traversal should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreApplyDryRunError::ArtifactPathEscapesBackup { .. }
    ));
}

// Ensure apply dry-runs reject status files that do not match the plan.
#[test]
fn apply_dry_run_rejects_mismatched_status() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let mut status = RestoreStatus::from_plan(&plan);
    status.backup_id = "other-backup".to_string();

    let err = RestoreApplyDryRun::try_from_plan(&plan, Some(&status))
        .expect_err("mismatched status should fail");

    assert!(matches!(
        err,
        RestoreApplyDryRunError::StatusPlanMismatch {
            field: "backup_id",
            ..
        }
    ));
}

// Ensure role-level verification checks are counted once per matching member.
#[test]
fn plan_expands_role_verification_checks_per_matching_member() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members.push(fleet_member(
        "app",
        CHILD_TWO,
        Some(ROOT),
        IdentityMode::Relocatable,
        1,
    ));
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "app".to_string(),
            checks: vec![VerificationCheck {
                kind: "app-ready".to_string(),
                method: Some("ready".to_string()),
                roles: Vec::new(),
            }],
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    assert_eq!(plan.verification_summary.fleet_checks, 0);
    assert_eq!(plan.verification_summary.member_check_groups, 1);
    assert_eq!(plan.verification_summary.member_checks, 5);
    assert_eq!(plan.verification_summary.members_with_checks, 3);
    assert_eq!(plan.verification_summary.total_checks, 5);
}

// Ensure member verification role filters control concrete restore checks.
#[test]
fn plan_applies_member_verification_role_filters() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.fleet.members[0]
        .verification_checks
        .push(VerificationCheck {
            kind: "root-only-inline".to_string(),
            method: Some("wrong_member".to_string()),
            roles: vec!["root".to_string()],
        });
    manifest
        .verification
        .member_checks
        .push(MemberVerificationChecks {
            role: "app".to_string(),
            checks: vec![
                VerificationCheck {
                    kind: "app-role-check".to_string(),
                    method: Some("app_ready".to_string()),
                    roles: vec!["app".to_string()],
                },
                VerificationCheck {
                    kind: "root-filtered-check".to_string(),
                    method: Some("wrong_role".to_string()),
                    roles: vec!["root".to_string()],
                },
            ],
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let app = plan
        .ordered_members()
        .into_iter()
        .find(|member| member.role == "app")
        .expect("app member should be planned");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let app_verification_methods = dry_run.phases[0]
        .operations
        .iter()
        .filter(|operation| {
            operation.source_canister == CHILD
                && operation.operation == RestoreApplyOperationKind::VerifyMember
        })
        .filter_map(|operation| operation.verification_method.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(app.verification_checks.len(), 2);
    assert_eq!(
        app.verification_checks
            .iter()
            .map(|check| check.kind.as_str())
            .collect::<Vec<_>>(),
        ["call", "app-role-check"]
    );
    assert_eq!(plan.verification_summary.member_checks, 3);
    assert_eq!(plan.verification_summary.total_checks, 3);
    assert_eq!(dry_run.rendered_operations, 7);
    assert_eq!(app_verification_methods, ["canic_ready", "app_ready"]);
}

// Ensure mapped restores must cover every source member.
#[test]
fn mapped_restore_requires_complete_mapping() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mapping = RestoreMapping {
        members: vec![RestoreMappingEntry {
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
        }],
    };

    let err = RestorePlanner::plan(&manifest, Some(&mapping))
        .expect_err("incomplete mapping should fail");

    assert!(matches!(err, RestorePlanError::MissingMappingSource(_)));
}

// Ensure mappings cannot silently include canisters outside the manifest.
#[test]
fn mapped_restore_rejects_unknown_mapping_sources() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let unknown = "rdmx6-jaaaa-aaaaa-aaadq-cai";
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: TARGET.to_string(),
            },
            RestoreMappingEntry {
                source_canister: unknown.to_string(),
                target_canister: unknown.to_string(),
            },
        ],
    };

    let err = RestorePlanner::plan(&manifest, Some(&mapping))
        .expect_err("unknown mapping source should fail");

    assert!(matches!(err, RestorePlanError::UnknownMappingSource(_)));
}

// Ensure duplicate target mappings fail before a plan is produced.
#[test]
fn duplicate_mapping_targets_fail_validation() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mapping = RestoreMapping {
        members: vec![
            RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            },
            RestoreMappingEntry {
                source_canister: CHILD.to_string(),
                target_canister: ROOT.to_string(),
            },
        ],
    };

    let err =
        RestorePlanner::plan(&manifest, Some(&mapping)).expect_err("duplicate targets should fail");

    assert!(matches!(err, RestorePlanError::DuplicateMappingTarget(_)));
}

// Write one artifact and record its path and checksum in the test manifest.
fn set_member_artifact(
    manifest: &mut FleetBackupManifest,
    canister_id: &str,
    root: &Path,
    artifact_path: &str,
    bytes: &[u8],
) {
    let full_path = root.join(artifact_path);
    fs::create_dir_all(full_path.parent().expect("artifact parent")).expect("create parent");
    fs::write(&full_path, bytes).expect("write artifact");
    let checksum = ArtifactChecksum::from_bytes(bytes);
    let member = manifest
        .fleet
        .members
        .iter_mut()
        .find(|member| member.canister_id == canister_id)
        .expect("member should exist");
    member.source_snapshot.artifact_path = artifact_path.to_string();
    member.source_snapshot.checksum = Some(checksum.hash);
}

// Return a unique temporary directory for restore tests.
fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("{name}-{nanos}"))
}
