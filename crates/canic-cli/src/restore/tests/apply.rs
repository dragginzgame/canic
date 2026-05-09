use super::*;
use crate::test_support::temp_dir;
use canic_backup::restore::{RestoreApplyDryRun, RestorePlanner};
use std::{ffi::OsString, fs};

// Ensure restore apply dry-run writes ordered operations from a plan.
#[test]
fn run_restore_apply_dry_run_writes_operations() {
    let root = temp_dir("canic-cli-restore-apply-dry-run");
    fs::create_dir_all(&root).expect("create temp root");
    let plan_path = root.join("restore-plan.json");
    let out_path = root.join("restore-apply-dry-run.json");
    let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");

    fs::write(
        &plan_path,
        serde_json::to_vec(&plan).expect("serialize plan"),
    )
    .expect("write plan");

    run([
        OsString::from("apply"),
        OsString::from("--plan"),
        OsString::from(plan_path.as_os_str()),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("write apply dry-run");

    let dry_run: RestoreApplyDryRun =
        serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
            .expect("decode dry-run");
    let dry_run_json: serde_json::Value = serde_json::to_value(&dry_run).expect("encode dry-run");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(dry_run.dry_run_version, 1);
    assert_eq!(dry_run.backup_id.as_str(), "backup-test");
    assert!(dry_run.ready);
    assert_eq!(dry_run.member_count, 2);
    assert_eq!(dry_run.planned_snapshot_uploads, 2);
    assert_eq!(dry_run.planned_operations, 6);
    assert_eq!(dry_run.rendered_operations, 6);
    assert_eq!(dry_run_json["operation_counts"]["snapshot_uploads"], 2);
    assert_eq!(dry_run_json["operation_counts"]["snapshot_loads"], 2);
    assert_eq!(dry_run_json["operation_counts"]["member_verifications"], 2);
    assert_eq!(dry_run_json["operation_counts"]["fleet_verifications"], 0);
    assert_eq!(
        dry_run_json["operation_counts"]["verification_operations"],
        2
    );
    assert_eq!(
        dry_run_json["operations"][0]["operation"],
        "upload-snapshot"
    );
    assert_eq!(dry_run_json["operations"][2]["operation"], "verify-member");
    assert_eq!(dry_run_json["operations"][2]["verification_kind"], "status");
}

// Ensure restore apply dry-run can validate artifacts under a backup directory.
#[test]
fn run_restore_apply_dry_run_validates_backup_dir_artifacts() {
    let root = temp_dir("canic-cli-restore-apply-artifacts");
    fs::create_dir_all(&root).expect("create temp root");
    let plan_path = root.join("restore-plan.json");
    let out_path = root.join("restore-apply-dry-run.json");
    let journal_path = root.join("restore-apply-journal.json");
    let mut manifest = restore_ready_manifest();
    write_manifest_artifacts(&root, &mut manifest);
    let plan = RestorePlanner::plan(&manifest, None).expect("build plan");

    fs::write(
        &plan_path,
        serde_json::to_vec(&plan).expect("serialize plan"),
    )
    .expect("write plan");

    run([
        OsString::from("apply"),
        OsString::from("--plan"),
        OsString::from(plan_path.as_os_str()),
        OsString::from("--backup-dir"),
        OsString::from(root.as_os_str()),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--journal-out"),
        OsString::from(journal_path.as_os_str()),
    ])
    .expect("write apply dry-run");
    let dry_run: RestoreApplyDryRun =
        serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
            .expect("decode dry-run");
    let validation = dry_run
        .artifact_validation
        .expect("artifact validation should be present");
    let journal_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).expect("read journal"))
            .expect("decode journal");
    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(validation.checked_members, 2);
    assert!(validation.artifacts_present);
    assert!(validation.checksums_verified);
    assert_eq!(validation.members_with_expected_checksums, 2);
    assert_eq!(journal_json["ready"], true);
    assert_eq!(journal_json["operation_count"], 6);
    assert_eq!(journal_json["operation_counts"]["snapshot_uploads"], 2);
    assert_eq!(journal_json["operation_counts"]["snapshot_loads"], 2);
    assert_eq!(journal_json["operation_counts"]["member_verifications"], 2);
    assert_eq!(journal_json["operation_counts"]["fleet_verifications"], 0);
    assert_eq!(
        journal_json["operation_counts"]["verification_operations"],
        2
    );
    assert_eq!(journal_json["ready_operations"], 6);
    assert_eq!(journal_json["blocked_operations"], 0);
    assert_eq!(journal_json["operations"][0]["state"], "ready");
}
