use super::BackupCommandError;
use canic_backup::persistence::BackupLayout;

const EXECUTION_JOURNAL_FILE_NAME: &str = "backup-execution-journal.json";

pub(super) fn ensure_execution_journal_exists(
    layout: &BackupLayout,
) -> Result<(), BackupCommandError> {
    if layout.execution_journal_path().is_file() {
        return Ok(());
    }

    Err(BackupCommandError::BackupLayoutIncomplete {
        missing: EXECUTION_JOURNAL_FILE_NAME,
    })
}
