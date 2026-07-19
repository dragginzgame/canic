use super::*;

// Ensure apply dry-runs render ordered operations without mutating targets.
#[test]
fn apply_dry_run_renders_ordered_member_operations() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan).expect("build restore dry-run");

    assert_eq!(dry_run.dry_run_version, 1);
    assert_eq!(dry_run.backup_id.as_str(), "fbk_test_001");
    assert!(dry_run.ready);
    assert_eq!(dry_run.member_count, 2);
    assert_eq!(dry_run.planned_canister_stops, 2);
    assert_eq!(dry_run.planned_canister_starts, 2);
    assert_eq!(dry_run.planned_snapshot_uploads, 2);
    assert_eq!(dry_run.planned_snapshot_loads, 2);
    assert_eq!(dry_run.planned_verification_checks, 2);
    assert_eq!(dry_run.planned_operations, 10);
    assert_eq!(dry_run.rendered_operations, 10);
    assert_eq!(dry_run.operation_counts.canister_stops, 2);
    assert_eq!(dry_run.operation_counts.canister_starts, 2);
    assert_eq!(dry_run.operation_counts.snapshot_uploads, 2);
    assert_eq!(dry_run.operation_counts.snapshot_loads, 2);
    assert_eq!(dry_run.operation_counts.member_verifications, 2);
    assert_eq!(dry_run.operation_counts.deployment_verifications, 0);
    assert_eq!(dry_run.operation_counts.verification_operations, 2);

    let operations = &dry_run.operations;
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
        operations[0]
            .artifact_checksum
            .as_ref()
            .map(|checksum| checksum.hash.as_str()),
        Some(HASH)
    );
    assert_eq!(
        operations[2].operation,
        RestoreApplyOperationKind::StopCanister
    );
    assert_eq!(
        operations[4].operation,
        RestoreApplyOperationKind::LoadSnapshot
    );
    assert_eq!(
        operations[6].operation,
        RestoreApplyOperationKind::StartCanister
    );
    assert_eq!(operations[6].source_canister, CHILD);
    assert_eq!(
        operations[8].operation,
        RestoreApplyOperationKind::VerifyMember
    );
    assert_eq!(operations[8].verification_kind, Some("status".to_string()));
    assert_eq!(operations[9].source_canister, CHILD);
    assert_eq!(
        operations[9].operation,
        RestoreApplyOperationKind::VerifyMember
    );
}

#[test]
fn apply_dry_run_rejects_invalid_plan() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let mut plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    plan.plan_version = 2;

    let err = RestoreApplyDryRun::from_plan(&plan).expect_err("invalid plan rejects");

    std::assert_matches!(err, RestorePlanError::UnsupportedVersion(2));
}

#[test]
fn apply_dry_run_requires_current_lifecycle_and_count_fields() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan).expect("build restore dry-run");

    for path in [
        &["planned_canister_stops"][..],
        &["planned_canister_starts"][..],
        &["operation_counts"][..],
        &["operation_counts", "canister_stops"][..],
        &["operation_counts", "canister_starts"][..],
    ] {
        let mut value = serde_json::to_value(&dry_run).expect("serialize dry-run");
        remove_json_field(&mut value, path);

        let err = serde_json::from_value::<RestoreApplyDryRun>(value)
            .expect_err("current apply dry-run field must be present");

        assert!(err.is_data());
    }
}

// Ensure apply dry-runs append deployment verification after member operations.
#[test]
fn apply_dry_run_renders_deployment_verification_operations() {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest
        .verification
        .deployment_checks
        .push(VerificationCheck {
            kind: "status".to_string(),
            roles: Vec::new(),
        });

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan).expect("build restore dry-run");

    assert_eq!(plan.operation_summary.planned_verification_checks, 3);
    assert_eq!(dry_run.rendered_operations, 11);
    let operation = dry_run
        .operations
        .last()
        .expect("deployment verification operation should be rendered");
    assert_eq!(operation.sequence, 10);
    assert_eq!(
        operation.operation,
        RestoreApplyOperationKind::VerifyDeployment
    );
    assert_eq!(operation.source_canister, ROOT);
    assert_eq!(operation.target_canister, ROOT);
    assert_eq!(operation.role, "deployment");
    assert_eq!(operation.snapshot_id, None);
    assert_eq!(operation.artifact_path, None);
    assert_eq!(operation.verification_kind, Some("status".to_string()));
}

// Ensure apply dry-run operation sequences remain unique in topology order.
#[test]
fn apply_dry_run_sequences_operations_in_topology_order() {
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan).expect("build restore dry-run");

    assert_eq!(dry_run.rendered_operations, 10);
    assert_eq!(dry_run.operations[0].sequence, 0);
    assert_eq!(dry_run.operations[2].sequence, 2);
    assert_eq!(dry_run.operations[5].sequence, 5);
    assert_eq!(dry_run.operations[9].sequence, 9);
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
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
    manifest.deployment.members[0].source_snapshot.artifact_path = "missing-child".to_string();

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let err = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect_err("missing artifact should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(err, RestoreApplyDryRunError::ArtifactMissing { .. });
}

// Ensure apply dry-runs reject artifact paths that escape the backup directory.
#[test]
fn apply_dry_run_rejects_artifact_path_traversal() {
    let root = temp_dir("canic-restore-apply-artifacts-traversal");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    manifest.deployment.members[1].source_snapshot.artifact_path = "../outside".to_string();

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let err = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect_err("path traversal should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        RestoreApplyDryRunError::ArtifactPathEscapesBackup { .. }
    );
}

#[cfg(unix)]
#[test]
fn apply_dry_run_rejects_symlinked_artifact_components() {
    let root = temp_dir("canic-restore-apply-artifact-symlink");
    fs::create_dir_all(root.join("artifacts")).expect("create artifact root");
    let outside = root.join("outside");
    fs::write(&outside, b"outside").expect("write outside artifact");
    std::os::unix::fs::symlink(&outside, root.join("artifacts/root"))
        .expect("create artifact symlink");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    let member = manifest
        .deployment
        .members
        .iter_mut()
        .find(|member| member.canister_id == ROOT)
        .expect("root member");
    member.source_snapshot.artifact_path = "artifacts/root".to_string();
    member.source_snapshot.checksum = Some(ArtifactChecksum::from_bytes(b"outside").hash);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let error = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect_err("symlinked artifact must reject");

    std::assert_matches!(error, RestoreApplyDryRunError::ArtifactUnsafeType { .. });
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn apply_dry_run_rejects_symlinked_backup_root() {
    let root = temp_dir("canic-restore-apply-root-symlink");
    let actual = root.join("actual");
    fs::create_dir_all(&actual).expect("create actual root");
    let linked = root.join("linked");
    std::os::unix::fs::symlink(&actual, &linked).expect("create root symlink");
    let manifest = valid_manifest(IdentityMode::Relocatable);
    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

    let error = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &linked)
        .expect_err("symlinked backup root must reject");

    std::assert_matches!(error, RestoreApplyDryRunError::ArtifactRootUnsafe { .. });
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn apply_dry_run_rejects_special_artifact_files() {
    use std::os::unix::net::UnixListener;

    // Keep the fixture path short enough for Unix-domain socket path limits.
    let root = temp_dir("cba-special");
    fs::create_dir_all(root.join("artifacts")).expect("create artifact root");
    let socket = root.join("artifacts/root");
    let listener = UnixListener::bind(&socket).expect("create artifact socket");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    let member = manifest
        .deployment
        .members
        .iter_mut()
        .find(|member| member.canister_id == ROOT)
        .expect("root member");
    member.source_snapshot.artifact_path = "artifacts/root".to_string();

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let error = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect_err("special artifact must reject");

    std::assert_matches!(error, RestoreApplyDryRunError::ArtifactUnsafeType { .. });
    drop(listener);
    fs::remove_file(socket).expect("remove artifact socket");
    fs::remove_dir_all(root).expect("remove fixture");
}
