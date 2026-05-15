use super::{BackupCommandError, BackupVerifyOptions};
use crate::backup::{labels::execution_is_complete, reference::resolve_backup_dir};
use canic_backup::persistence::{BackupIntegrityReport, BackupLayout};

pub(super) fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    if !layout.manifest_path().is_file() && layout.backup_plan_path().is_file() {
        let plan = layout.read_backup_plan()?;
        return Err(BackupCommandError::DryRunNotComplete {
            plan_id: plan.plan_id,
        });
    }
    if layout.backup_plan_path().is_file() {
        let plan = layout.read_backup_plan()?;
        let journal = layout.read_execution_journal()?;
        layout.verify_execution_integrity()?;
        if !execution_is_complete(&journal.resume_summary()) {
            return Err(BackupCommandError::DryRunNotComplete {
                plan_id: plan.plan_id,
            });
        }
    }

    layout.verify_integrity().map_err(BackupCommandError::from)
}
