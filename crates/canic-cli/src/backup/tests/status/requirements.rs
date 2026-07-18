//! Module: backup::tests::status::requirements
//!
//! Responsibility: status completion requirement tests.
//! Does not own: status layout rendering or option parsing.
//! Boundary: `--require-complete` enforcement for backup status reports.

use super::super::super::*;
use super::super::fixtures::*;
use canic_backup::execution::BackupExecutionJournal;
use std::path::PathBuf;

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
        layout_status: BackupExecutionLayoutStatus::DryRun,
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        deployment: plan.fleet.clone(),
        environment: plan.environment.clone(),
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
