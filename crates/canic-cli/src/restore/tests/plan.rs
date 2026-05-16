use super::*;
use crate::test_support::temp_dir;
use canic_backup::{persistence::BackupLayout, restore::RestorePlan};
use serde_json::json;
use std::{ffi::OsString, fs};

// Ensure backup-dir restore planning reads the canonical layout manifest.
#[test]
fn plan_restore_reads_manifest_from_backup_dir() {
    let root = temp_dir("canic-cli-restore-plan-layout");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");

    let options = RestorePlanOptions {
        backup_ref: None,
        manifest: None,
        backup_dir: Some(root.clone()),
        mapping: None,
        out: None,
        require_verified: false,
        require_restore_ready: false,
    };

    let plan = plan_restore(&options).expect("plan restore");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(plan.backup_id, "backup-test");
    assert_eq!(plan.member_count, 2);
}

// Ensure restore planning has exactly one manifest source.
#[test]
fn parse_rejects_conflicting_manifest_sources() {
    let err = RestorePlanOptions::parse([
        OsString::from("--manifest"),
        OsString::from("manifest.json"),
        OsString::from("--backup-dir"),
        OsString::from("backups/run"),
    ])
    .expect_err("conflicting sources should fail");

    assert!(matches!(err, RestoreCommandError::Usage(_)));
}

// Ensure verified planning requires the canonical backup layout source.
#[test]
fn parse_rejects_require_verified_with_manifest_source() {
    let err = RestorePlanOptions::parse([
        OsString::from("--manifest"),
        OsString::from("manifest.json"),
        OsString::from("--require-verified"),
    ])
    .expect_err("verification should require a backup layout");

    assert!(matches!(err, RestoreCommandError::Usage(_)));
}

// Ensure restore planning can require manifest, journal, and artifact integrity.
#[test]
fn plan_restore_requires_verified_backup_layout() {
    let root = temp_dir("canic-cli-restore-plan-verified");
    let layout = BackupLayout::new(root.clone());
    let manifest = valid_manifest();
    write_verified_layout(&root, &layout, &manifest);

    let options = RestorePlanOptions {
        backup_ref: None,
        manifest: None,
        backup_dir: Some(root.clone()),
        mapping: None,
        out: None,
        require_verified: true,
        require_restore_ready: false,
    };

    let plan = plan_restore(&options).expect("plan verified restore");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(plan.backup_id, "backup-test");
    assert_eq!(plan.member_count, 2);
}

// Ensure required verification fails before planning when the layout is incomplete.
#[test]
fn plan_restore_rejects_unverified_backup_layout() {
    let root = temp_dir("canic-cli-restore-plan-unverified");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");

    let options = RestorePlanOptions {
        backup_ref: None,
        manifest: None,
        backup_dir: Some(root.clone()),
        mapping: None,
        out: None,
        require_verified: true,
        require_restore_ready: false,
    };

    let err = plan_restore(&options).expect_err("missing journal should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(err, RestoreCommandError::Persistence(_)));
}

// Ensure the CLI planning path validates manifests and applies mappings.
#[test]
fn plan_restore_reads_manifest_and_mapping() {
    let root = temp_dir("canic-cli-restore-plan");
    fs::create_dir_all(&root).expect("create temp root");
    let manifest_path = root.join("manifest.json");
    let mapping_path = root.join("mapping.json");

    fs::write(
        &manifest_path,
        serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
    )
    .expect("write manifest");
    fs::write(
        &mapping_path,
        json!({
            "members": [
                {"source_canister": ROOT, "target_canister": ROOT},
                {"source_canister": CHILD, "target_canister": MAPPED_CHILD}
            ]
        })
        .to_string(),
    )
    .expect("write mapping");

    let options = RestorePlanOptions {
        backup_ref: None,
        manifest: Some(manifest_path),
        backup_dir: None,
        mapping: Some(mapping_path),
        out: None,
        require_verified: false,
        require_restore_ready: false,
    };

    let plan = plan_restore(&options).expect("plan restore");

    fs::remove_dir_all(root).expect("remove temp root");
    let members = plan.ordered_members();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].source_canister, ROOT);
    assert_eq!(members[1].target_canister, MAPPED_CHILD);
}

// Ensure restore-readiness gating happens after writing the plan artifact.
#[test]
fn run_restore_plan_require_restore_ready_writes_plan_then_fails() {
    let root = temp_dir("canic-cli-restore-plan-require-ready");
    fs::create_dir_all(&root).expect("create temp root");
    let manifest_path = root.join("manifest.json");
    let out_path = root.join("plan.json");

    fs::write(
        &manifest_path,
        serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
    )
    .expect("write manifest");

    let err = run([
        OsString::from("plan"),
        OsString::from("--manifest"),
        OsString::from(manifest_path.as_os_str()),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--require-restore-ready"),
    ])
    .expect_err("restore readiness should be enforced");

    assert!(out_path.exists());
    let plan: RestorePlan =
        serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(!plan.readiness_summary.ready);
    assert!(matches!(
        err,
        RestoreCommandError::RestoreNotReady {
            reasons,
            ..
        } if reasons == ["missing-snapshot-checksum"]
    ));
}

// Ensure restore-readiness gating accepts plans with complete snapshot artifacts.
#[test]
fn run_restore_plan_require_restore_ready_accepts_ready_plan() {
    let root = temp_dir("canic-cli-restore-plan-ready");
    fs::create_dir_all(&root).expect("create temp root");
    let manifest_path = root.join("manifest.json");
    let out_path = root.join("plan.json");

    fs::write(
        &manifest_path,
        serde_json::to_vec(&restore_ready_manifest()).expect("serialize manifest"),
    )
    .expect("write manifest");

    run([
        OsString::from("plan"),
        OsString::from("--manifest"),
        OsString::from(manifest_path.as_os_str()),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--require-restore-ready"),
    ])
    .expect("restore-ready plan should pass");

    let plan: RestorePlan =
        serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(plan.readiness_summary.ready);
    assert!(plan.readiness_summary.reasons.is_empty());
}

// Ensure restore prepare writes the default plan and journal beside a backup layout.
#[test]
fn run_restore_prepare_writes_default_layout_artifacts() {
    let root = temp_dir("canic-cli-restore-prepare");
    let layout = BackupLayout::new(root.clone());
    let mut manifest = restore_ready_manifest();
    write_manifest_artifacts(&root, &mut manifest);
    layout.write_manifest(&manifest).expect("write manifest");
    let out_path = root.join("restore-prepare.json");

    run([
        OsString::from("prepare"),
        OsString::from("--backup-dir"),
        OsString::from(root.as_os_str()),
        OsString::from("--require-restore-ready"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("prepare restore");

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).expect("read prepare report"))
            .expect("decode report");
    let plan_path = root.join("restore-plan.json");
    let journal_path = root.join("restore-apply-journal.json");
    let plan: RestorePlan =
        serde_json::from_slice(&fs::read(&plan_path).expect("read plan")).expect("decode plan");
    let journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).expect("read journal"))
            .expect("decode journal");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report["backup_id"], "backup-test");
    assert_eq!(report["ready"], true);
    assert_eq!(report["members"], 2);
    assert_eq!(report["operations"], 10);
    assert!(plan.readiness_summary.ready);
    assert_eq!(journal["ready"], true);
    assert_eq!(journal["operation_count"], 10);
}

// Ensure prepared backup references fail with an operator action, not raw IO.
#[test]
fn prepared_plan_path_reports_prepare_action_when_missing() {
    let root = temp_dir("canic-cli-restore-missing-plan");
    let path = root.join("restore-plan.json");

    let err = require_prepared_plan_path("1", path.clone()).expect_err("missing plan rejects");

    fs::remove_dir_all(root).ok();
    assert!(matches!(
        err,
        RestoreCommandError::PreparedPlanMissing {
            backup_ref,
            path: missing_path,
        } if backup_ref == "1" && missing_path == path.display().to_string()
    ));
}

// Ensure prepared runner references fail with an operator action, not raw IO.
#[test]
fn prepared_journal_path_reports_prepare_action_when_missing() {
    let root = temp_dir("canic-cli-restore-missing-journal");
    let path = root.join("restore-apply-journal.json");

    let err =
        require_prepared_journal_path("1", path.clone()).expect_err("missing journal rejects");

    fs::remove_dir_all(root).ok();
    assert!(matches!(
        err,
        RestoreCommandError::PreparedJournalMissing {
            backup_ref,
            path: missing_path,
        } if backup_ref == "1" && missing_path == path.display().to_string()
    ));
}
