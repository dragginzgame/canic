use super::*;

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
