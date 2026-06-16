//! Module: backup::tests
//!
//! Responsibility: backup command behavior tests.
//! Does not own: backup implementation or shared fixture construction.
//! Boundary: integration-style unit tests for the `canic backup` command family.

mod fixtures;
mod options;
mod status;

use super::*;
use crate::test_support::temp_dir;
use canic_backup::{execution::BackupExecutionJournal, persistence::BackupLayout};
use fixtures::*;
use std::{fs, path::Path};

// Ensure dry-run persistence writes a plan and matching execution journal.
#[test]
fn backup_create_dry_run_persists_plan_and_execution_journal() {
    let root = temp_dir("canic-cli-backup-create-plan");
    let plan = valid_backup_plan();

    let persisted = persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let layout = BackupLayout::new(root.clone());
    let read_plan = layout.read_backup_plan().expect("read backup plan");
    let journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let report = layout
        .verify_execution_integrity()
        .expect("verify execution integrity");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(persisted.plan_id, plan.plan_id);
    assert_eq!(read_plan.plan_id, plan.plan_id);
    assert_eq!(journal.plan_id, plan.plan_id);
    assert!(report.verified);
}

// Ensure dry-run persistence reports whether it created or reused a layout.
#[test]
fn backup_create_persistence_reports_layout_source() {
    let root = temp_dir("canic-cli-backup-create-layout-source");
    let plan = valid_backup_plan();

    let (created, created_from_existing) =
        persist_backup_create_dry_run_with_layout(&root, &plan).expect("persist new layout");
    let (resumed, resumed_from_existing) =
        persist_backup_create_dry_run_with_layout(&root, &plan).expect("reuse existing layout");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(created.plan_id, plan.plan_id);
    assert_eq!(resumed.plan_id, plan.plan_id);
    assert!(!created_from_existing);
    assert!(resumed_from_existing);
}

// Ensure backup create uses an existing output layout as the resume boundary.
#[test]
fn backup_create_persistence_preserves_existing_execution_layout() {
    let root = temp_dir("canic-cli-backup-create-resume");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist initial plan");
    let layout = BackupLayout::new(root.clone());
    let mut journal = accepted_execution_journal();
    complete_execution_operation(&mut journal, 4);
    layout
        .write_execution_journal(&journal)
        .expect("write progressed execution journal");
    let mut replacement = valid_backup_plan();
    replacement.plan_id = "plan-replacement".to_string();
    replacement.run_id = "run-replacement".to_string();

    let resumed =
        persist_backup_create_dry_run(&root, &replacement).expect("reuse existing layout");
    let read_plan = layout.read_backup_plan().expect("read backup plan");
    let read_journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let summary = read_journal.resume_summary();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(resumed.plan_id, plan.plan_id);
    assert_eq!(read_plan.plan_id, plan.plan_id);
    assert_eq!(summary.completed_operations, 5);
    assert_eq!(summary.next_operation.expect("next operation").sequence, 5);
}

// Ensure backup create does not reuse an output layout for a different request.
#[test]
fn backup_create_persistence_rejects_incompatible_existing_layout() {
    let root = temp_dir("canic-cli-backup-create-incompatible-resume");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist initial plan");
    let mut requested = valid_backup_plan();
    requested.network = "ic".to_string();

    let err = persist_backup_create_dry_run(&root, &requested)
        .expect_err("incompatible existing layout rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutMismatch {
            field: "network",
            existing,
            requested,
        } if existing == "local" && requested == "ic"
    );
}

// Ensure dry-run layouts cannot be reused as executable backup layouts.
#[test]
fn backup_create_persistence_rejects_dry_run_layout_for_execute_request() {
    let root = temp_dir("canic-cli-backup-create-dry-run-execute-mismatch");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");
    let requested = valid_executable_backup_plan();

    let err = persist_backup_create_dry_run(&root, &requested)
        .expect_err("dry-run layout rejects execute request");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutMismatch {
            field: "requires_root_controller",
            existing,
            requested,
        } if existing == "true" && requested == "false"
    );
}

// Ensure completed execution layouts do not synthesize a missing execution journal.
#[test]
fn backup_create_persistence_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-create-missing-execution-journal");
    let plan = valid_backup_plan();
    write_manifest_plan_without_execution_journal(&root);

    let err = persist_backup_create_dry_run(&root, &plan)
        .expect_err("manifest layout missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}

// Ensure backup inspect reads dry-run plan and execution details.
#[test]
fn backup_inspect_reads_dry_run_details() {
    let root = temp_dir("canic-cli-backup-inspect-dry-run");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let options = BackupInspectOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        json: false,
    };
    let report = backup_inspect(&options).expect("inspect dry-run");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.layout_status, "dry-run");
    assert_eq!(report.plan_id, plan.plan_id);
    assert_eq!(report.targets.len(), 1);
    assert_eq!(report.targets[0].expected_module_hash, HASH);
    assert_eq!(report.operations.len(), 10);
}

// Ensure backup inspect reports incomplete execution-backed layouts clearly.
#[test]
fn backup_inspect_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-inspect-missing-execution-journal");
    write_manifest_plan_without_execution_journal(&root);

    let options = BackupInspectOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        json: false,
    };
    let err = backup_inspect(&options).expect_err("missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}

// Ensure backup list scans manifest-bearing directories and renders reusable paths.
#[test]
fn backup_list_reads_backup_directories() {
    let root = temp_dir("canic-cli-backup-list");
    let first = root.join("deployment-demo-20260507-120000");
    let second = root.join("deployment-demo-20260507-130000");
    let planned = root.join("deployment-demo-20260511-001234");
    let ignored = root.join("not-a-backup");

    BackupLayout::new(first)
        .write_manifest(&valid_manifest_with("backup-old", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second)
        .write_manifest(&valid_manifest_with("backup-new", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");
    let mut plan = valid_backup_plan();
    plan.plan_id = "plan-demo-20260511-001234".to_string();
    plan.run_id = "run-demo-20260511-001234".to_string();
    let planned_layout = BackupLayout::new(planned);
    planned_layout
        .write_backup_plan(&plan)
        .expect("write planned backup");
    planned_layout
        .write_execution_journal(
            &BackupExecutionJournal::from_plan(&plan).expect("execution journal"),
        )
        .expect("write planned journal");
    fs::create_dir_all(&ignored).expect("create ignored dir");

    let options = BackupListOptions {
        dir: root.clone(),
        out: None,
    };
    let entries = backup_list(&options).expect("list backups");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().any(|entry| entry.backup_id == "backup-new"));
    assert!(entries.iter().any(|entry| entry.backup_id == "backup-old"));
    let dry_run = entries
        .iter()
        .find(|entry| entry.backup_id == "plan-demo-20260511-001234")
        .expect("dry-run entry");
    assert_eq!(dry_run.status, "dry-run");
    assert_eq!(dry_run.members, 1);
    assert_eq!(dry_run.created_at, unix_marker_for_stamp("20260511-001234"));
}

// Ensure backup list reports execution-backed manifest layouts by execution state.
#[test]
fn backup_list_reports_execution_backed_manifest_status() {
    let root = temp_dir("canic-cli-backup-list-execution-status");
    let running = root.join("deployment-demo-20260507-140000");
    let complete = root.join("deployment-demo-20260507-150000");
    let invalid = root.join("deployment-demo-20260507-160000");
    let missing_journal = root.join("deployment-demo-20260507-170000");
    let checksum = write_artifact(&complete, b"root artifact");

    write_manifest_plan_journal(&running, accepted_execution_journal());

    let mut complete_journal = accepted_execution_journal();
    for sequence in 4..=9 {
        complete_execution_operation(&mut complete_journal, sequence);
    }
    write_manifest_plan_journal(&complete, complete_journal);
    BackupLayout::new(complete.clone())
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write download journal");

    let mut invalid_journal = accepted_execution_journal();
    invalid_journal.operations[0].operation_id = "different-operation".to_string();
    write_manifest_plan_journal(&invalid, invalid_journal);
    let missing_layout = BackupLayout::new(missing_journal.clone());
    missing_layout
        .write_manifest(&valid_manifest())
        .expect("write missing-journal manifest");
    missing_layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write missing-journal plan");

    let entries = backup_list(&BackupListOptions {
        dir: root.clone(),
        out: None,
    })
    .expect("list backups");

    fs::remove_dir_all(root).expect("remove temp root");
    let status_for = |dir: &Path| {
        entries
            .iter()
            .find(|entry| entry.dir == dir)
            .map(|entry| entry.status.as_str())
            .expect("entry exists")
    };
    assert_eq!(status_for(&running), "running");
    assert_eq!(status_for(&complete), "complete");
    assert_eq!(status_for(&invalid), "invalid-plan-journal");
    assert_eq!(status_for(&missing_journal), "invalid-plan-journal");
}

// Ensure short backup references resolve through the same ordering as backup list.
#[test]
fn backup_reference_resolves_rows_and_backup_ids() {
    let root = temp_dir("canic-cli-backup-reference");
    let first = root.join("deployment-demo-20260507-120000");
    let second = root.join("deployment-demo-20260507-130000");

    BackupLayout::new(first.clone())
        .write_manifest(&valid_manifest_with("backup-old", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second.clone())
        .write_manifest(&valid_manifest_with("backup-new", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");

    let by_row = resolve_backup_reference_in(&root, "1").expect("resolve row");
    let by_id = resolve_backup_reference_in(&root, "backup-old").expect("resolve id");
    let missing = resolve_backup_reference_in(&root, "99").expect_err("missing row rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(by_row, second);
    assert_eq!(by_id, first);
    std::assert_matches!(missing, BackupCommandError::BackupReferenceNotFound { .. });
}

// Ensure duplicate backup ids fail closed instead of resolving arbitrarily.
#[test]
fn backup_reference_rejects_ambiguous_backup_ids() {
    let root = temp_dir("canic-cli-backup-reference-ambiguous");
    let first = root.join("deployment-demo-20260507-120000");
    let second = root.join("deployment-demo-20260507-130000");

    BackupLayout::new(first)
        .write_manifest(&valid_manifest_with("backup-same", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second)
        .write_manifest(&valid_manifest_with("backup-same", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");

    let err = resolve_backup_reference_in(&root, "backup-same").expect_err("ambiguous rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(err, BackupCommandError::BackupReferenceAmbiguous { .. });
}

// Ensure unfinished execution layouts use the journal timestamp, not a raw run-id stamp.
#[test]
fn backup_list_uses_execution_journal_timestamp_for_planned_layouts() {
    let root = temp_dir("canic-cli-backup-list-created-at-journal");
    let planned = root.join("deployment-demo-20260511-001234");
    let mut plan = valid_backup_plan();
    plan.plan_id = "plan-demo-20260511-001234".to_string();
    plan.run_id = "run-demo-20260511-001234".to_string();
    let layout = BackupLayout::new(planned);
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&accepted_execution_journal())
        .expect("write execution journal");

    let entries = backup_list(&BackupListOptions {
        dir: root.clone(),
        out: None,
    })
    .expect("list backups");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].created_at, "unix:10");
}

// Ensure prune previews and removes only explicitly selected failed backup directories.
#[test]
fn backup_prune_removes_failed_layouts() {
    let root = temp_dir("canic-cli-backup-prune-failed");
    let failed = root.join("deployment-demo-20260511-001234");
    let complete = root.join("deployment-demo-20260511-010000");
    let failed_layout = BackupLayout::new(failed.clone());
    let mut journal = accepted_execution_journal();
    fail_execution_operation(&mut journal, 4, "simulated failure");
    failed_layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write failed plan");
    failed_layout
        .write_execution_journal(&journal)
        .expect("write failed journal");
    BackupLayout::new(complete.clone())
        .write_manifest(&valid_manifest_with("backup-complete", "unix:1778457600"))
        .expect("write complete manifest");

    let dry_run = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        failed: true,
        keep: None,
        dry_run: true,
        out: None,
    })
    .expect("dry-run prune");
    assert_eq!(dry_run.scanned, 2);
    assert_eq!(dry_run.selected, 1);
    assert_eq!(dry_run.pruned, 0);
    assert_eq!(dry_run.entries[0].action, "would-remove");
    assert!(failed.is_dir());

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        failed: true,
        keep: None,
        dry_run: false,
        out: None,
    })
    .expect("execute prune");

    assert_eq!(report.pruned, 1);
    assert!(!failed.is_dir());
    assert!(complete.is_dir());
    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure keep-based pruning uses the same newest-first ordering as backup list.
#[test]
fn backup_prune_keep_removes_older_entries() {
    let root = temp_dir("canic-cli-backup-prune-keep");
    let newest = root.join("deployment-demo-20260511-020000");
    let middle = root.join("deployment-demo-20260511-010000");
    let oldest = root.join("deployment-demo-20260511-000000");
    BackupLayout::new(newest.clone())
        .write_manifest(&valid_manifest_with("backup-newest", "unix:1778464800"))
        .expect("write newest manifest");
    BackupLayout::new(middle.clone())
        .write_manifest(&valid_manifest_with("backup-middle", "unix:1778461200"))
        .expect("write middle manifest");
    BackupLayout::new(oldest.clone())
        .write_manifest(&valid_manifest_with("backup-oldest", "unix:1778457600"))
        .expect("write oldest manifest");

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        failed: false,
        keep: Some(2),
        dry_run: false,
        out: None,
    })
    .expect("prune old backups");

    assert_eq!(report.scanned, 3);
    assert_eq!(report.pruned, 1);
    assert_eq!(report.entries[0].backup_id, "backup-oldest");
    assert!(newest.is_dir());
    assert!(middle.is_dir());
    assert!(!oldest.is_dir());
    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure verification rejects dry-run plans with a backup-specific error.
#[test]
fn verify_backup_rejects_dry_run_layout() {
    let root = temp_dir("canic-cli-backup-verify-dry-run");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("dry-run verify rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::DryRunNotComplete { plan_id } if plan_id == "plan-test"
    );
}

// Ensure verification reports incomplete execution-backed layouts clearly.
#[test]
fn verify_backup_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-verify-missing-execution-journal");
    write_manifest_plan_without_execution_journal(&root);

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}

// Ensure verification rejects execution-backed layouts that finalized artifacts before execution completion.
#[test]
fn verify_backup_rejects_incomplete_execution_layout_with_manifest() {
    let root = temp_dir("canic-cli-backup-verify-incomplete-execution");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&accepted_execution_journal())
        .expect("write incomplete execution journal");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("incomplete execution rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::DryRunNotComplete { plan_id } if plan_id == "plan-test"
    );
}

// Ensure verification rejects execution-backed layouts whose plan and execution journal drift.
#[test]
fn verify_backup_rejects_execution_plan_journal_mismatch() {
    let root = temp_dir("canic-cli-backup-verify-execution-mismatch");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut journal = accepted_execution_journal();
    journal.operations[0].operation_id = "different-operation".to_string();

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write mismatched execution journal");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("mismatched execution rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::Persistence(
            canic_backup::persistence::PersistenceError::PlanJournalOperationMismatch {
                field: "operation_id",
                ..
            }
        )
    );
}

// Ensure the CLI verification path reads a layout and returns an integrity report.
#[test]
fn verify_backup_reads_layout_and_artifacts() {
    let root = temp_dir("canic-cli-backup-verify");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash.clone()))
        .expect("write journal");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let report = verify_backup(&options).expect("verify backup");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.backup_id, "backup-test");
    assert!(report.verified);
    assert_eq!(report.durable_artifacts, 1);
    assert_eq!(report.artifacts[0].checksum, checksum.hash);
}
