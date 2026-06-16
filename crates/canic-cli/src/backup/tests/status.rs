//! Module: backup::tests::status
//!
//! Responsibility: backup status and completion-gate behavior tests.
//! Does not own: backup persistence fixtures or command option parsing.
//! Boundary: status report behavior for download and execution-backed layouts.

use super::super::*;
use super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::{execution::BackupExecutionJournal, persistence::BackupLayout};
use std::{fs, path::PathBuf};

// Ensure backup status reads the journal and reports resume actions.
#[test]
fn backup_status_reads_journal_resume_report() {
    let root = temp_dir("canic-cli-backup-status");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_journal(&journal_with_checksum(HASH.to_string()))
        .expect("write journal");

    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read backup status");

    fs::remove_dir_all(root).expect("remove temp root");
    let BackupStatusReport::Download(report) = report else {
        panic!("expected download status");
    };
    assert_eq!(report.backup_id, "backup-test");
    assert_eq!(report.total_artifacts, 1);
    assert!(report.is_complete);
    assert_eq!(report.pending_artifacts, 0);
    assert_eq!(report.counts.skip, 1);
}

// Ensure backup status can summarize dry-run plan/execution layouts.
#[test]
fn backup_status_reads_dry_run_execution_summary() {
    let root = temp_dir("canic-cli-backup-status-dry-run");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read dry-run status");

    fs::remove_dir_all(root).expect("remove temp root");
    let BackupStatusReport::DryRun(report) = report else {
        panic!("expected dry-run status");
    };
    assert_eq!(report.layout_status, "dry-run");
    assert_eq!(report.plan_id, plan.plan_id);
    assert_eq!(report.targets, 1);
    assert_eq!(report.execution.plan_id, plan.plan_id);
    assert!(!report.execution.preflight_accepted);
    assert!(report.execution.blocked_operations > 0);
}

// Ensure backup status reports incomplete execution-backed layouts clearly.
#[test]
fn backup_status_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-status-missing-execution-journal");
    write_manifest_plan_without_execution_journal(&root);

    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let err = backup_status(&options).expect_err("missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}

// Ensure backup status reports an execution layout as running once preflight is accepted.
#[test]
fn backup_status_reports_running_execution_layout() {
    let report = backup_status_for_execution_journal(
        "canic-cli-backup-status-running",
        accepted_execution_journal(),
        false,
    );

    assert_eq!(report.layout_status, "running");
    assert!(report.execution.preflight_accepted);
    assert_eq!(report.execution.failed_operations, 0);
    assert!(report.execution.ready_operations > 0);
}

// Ensure backup status reports failed execution journals without requiring a manifest.
#[test]
fn backup_status_reports_failed_execution_layout() {
    let mut journal = accepted_execution_journal();
    complete_execution_operation(&mut journal, 4);
    fail_execution_operation(&mut journal, 5, "snapshot failed");

    let report =
        backup_status_for_execution_journal("canic-cli-backup-status-failed", journal, false);

    assert_eq!(report.layout_status, "failed");
    assert_eq!(report.execution.failed_operations, 1);
    assert_eq!(
        report
            .execution
            .next_operation
            .expect("failed operation")
            .sequence,
        5
    );
}

// Ensure backup status reports complete only when the execution journal is complete and a manifest exists.
#[test]
fn backup_status_reports_complete_execution_layout() {
    let mut journal = accepted_execution_journal();
    for sequence in 4..=9 {
        complete_execution_operation(&mut journal, sequence);
    }

    let report =
        backup_status_for_execution_journal("canic-cli-backup-status-complete", journal, true);

    assert_eq!(report.layout_status, "complete");
    assert_eq!(
        report.execution.completed_operations + report.execution.skipped_operations,
        report.execution.total_operations
    );
}

// Ensure require-complete does not accept completed execution state before manifest finalization.
#[test]
fn require_complete_rejects_complete_execution_without_manifest() {
    let mut journal = accepted_execution_journal();
    for sequence in 4..=9 {
        complete_execution_operation(&mut journal, sequence);
    }
    let report = BackupStatusReport::DryRun(backup_status_for_execution_journal(
        "canic-cli-backup-status-complete-no-manifest",
        journal,
        false,
    ));
    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(PathBuf::from("unused")),
        out: None,
        require_complete: true,
    };

    let err = enforce_status_requirements(&options, &report)
        .expect_err("complete execution without manifest should fail");

    std::assert_matches!(
        err,
        BackupCommandError::DryRunNotComplete { plan_id } if plan_id == "plan-test"
    );
}

// Ensure require-complete accepts already durable backup journals.
#[test]
fn require_complete_accepts_complete_status() {
    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(PathBuf::from("unused")),
        out: None,
        require_complete: true,
    };
    let report = journal_with_checksum(HASH.to_string()).resume_report();

    enforce_status_requirements(&options, &BackupStatusReport::Download(report))
        .expect("complete status should pass");
}

// Ensure require-complete rejects journals that still need resume work.
#[test]
fn require_complete_rejects_incomplete_status() {
    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(PathBuf::from("unused")),
        out: None,
        require_complete: true,
    };
    let report = created_journal().resume_report();

    let err = enforce_status_requirements(&options, &BackupStatusReport::Download(report))
        .expect_err("incomplete status should fail");

    std::assert_matches!(
        err,
        BackupCommandError::IncompleteJournal {
            pending_artifacts: 1,
            total_artifacts: 1,
            ..
        }
    );
}

// Ensure require-complete rejects dry-run layouts.
#[test]
fn require_complete_rejects_dry_run_status() {
    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(PathBuf::from("unused")),
        out: None,
        require_complete: true,
    };
    let plan = valid_backup_plan();
    let report = BackupStatusReport::DryRun(BackupDryRunStatusReport {
        layout_status: "dry-run".to_string(),
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        deployment: plan.fleet.clone(),
        network: plan.network.clone(),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
        execution: BackupExecutionJournal::from_plan(&plan)
            .expect("execution journal")
            .resume_summary(),
    });

    let err =
        enforce_status_requirements(&options, &report).expect_err("dry-run status should fail");

    std::assert_matches!(
        err,
        BackupCommandError::DryRunNotComplete { plan_id } if plan_id == "plan-test"
    );
}

// Ensure dry-run status JSON exposes deployment identity, not stale fleet identity.
#[test]
fn backup_dry_run_status_json_uses_deployment_identity_field() {
    let plan = valid_backup_plan();
    let report = BackupDryRunStatusReport {
        layout_status: "dry-run".to_string(),
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        deployment: plan.fleet.clone(),
        network: plan.network.clone(),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
        execution: BackupExecutionJournal::from_plan(&plan)
            .expect("execution journal")
            .resume_summary(),
    };

    let value = serde_json::to_value(&report).expect("serialize status report");

    assert_eq!(value["deployment"], "demo");
    assert!(value.get("fleet").is_none());
}
