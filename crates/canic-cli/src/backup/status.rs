use super::{
    BackupCommandError, BackupDryRunStatusReport, BackupStatusOptions, BackupStatusReport,
};
use crate::backup::{
    labels::{execution_is_complete, execution_layout_status},
    reference::resolve_backup_dir,
};
use canic_backup::persistence::BackupLayout;

pub(super) fn backup_status(
    options: &BackupStatusOptions,
) -> Result<BackupStatusReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    if layout.backup_plan_path().is_file() {
        let plan = layout.read_backup_plan()?;
        let journal = layout.read_execution_journal()?;
        layout.verify_execution_integrity()?;
        return Ok(BackupStatusReport::DryRun(BackupDryRunStatusReport {
            layout_status: execution_layout_status(&journal, layout.manifest_path().is_file()),
            plan_id: plan.plan_id.clone(),
            run_id: plan.run_id.clone(),
            fleet: plan.fleet,
            network: plan.network,
            targets: plan.targets.len(),
            operations: plan.phases.len(),
            execution: journal.resume_summary(),
        }));
    }
    if layout.journal_path().is_file() {
        let journal = layout.read_journal()?;
        return Ok(BackupStatusReport::Download(journal.resume_report()));
    }

    let journal = layout.read_journal()?;
    Ok(BackupStatusReport::Download(journal.resume_report()))
}

pub(super) fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &BackupStatusReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete {
        return Ok(());
    }

    ensure_complete_status(report)
}

fn ensure_complete_status(report: &BackupStatusReport) -> Result<(), BackupCommandError> {
    match report {
        BackupStatusReport::Download(report) if report.is_complete => Ok(()),
        BackupStatusReport::Download(report) => Err(BackupCommandError::IncompleteJournal {
            backup_id: report.backup_id.clone(),
            total_artifacts: report.total_artifacts,
            pending_artifacts: report.pending_artifacts,
        }),
        BackupStatusReport::DryRun(report)
            if report.layout_status == "complete" && execution_is_complete(&report.execution) =>
        {
            Ok(())
        }
        BackupStatusReport::DryRun(report) => Err(BackupCommandError::DryRunNotComplete {
            plan_id: report.plan_id.clone(),
        }),
    }
}
