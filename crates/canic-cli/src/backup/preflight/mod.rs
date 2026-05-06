pub use canic_backup::preflight::BackupPreflightReport;
use canic_backup::preflight::{BackupPreflightConfig, BackupPreflightError, run_backup_preflight};

use super::{BackupCommandError, BackupPreflightOptions};

/// Run all no-mutation backup checks and write standard preflight artifacts.
pub fn backup_preflight(
    options: &BackupPreflightOptions,
) -> Result<BackupPreflightReport, BackupCommandError> {
    let report = run_backup_preflight(&BackupPreflightConfig {
        backup_dir: options.dir.clone(),
        out_dir: options.out_dir.clone(),
        mapping: options.mapping.clone(),
    })
    .map_err(BackupCommandError::from)?;

    enforce_preflight_requirements(options, &report)?;
    Ok(report)
}

// Enforce caller-requested preflight requirements after all artifacts are written.
fn enforce_preflight_requirements(
    options: &BackupPreflightOptions,
    report: &BackupPreflightReport,
) -> Result<(), BackupCommandError> {
    if options.require_design_v1 && !report.manifest_design_v1_ready {
        return Err(BackupCommandError::DesignConformanceNotReady {
            backup_id: report.backup_id.clone(),
        });
    }

    if !options.require_restore_ready || report.restore_ready {
        return Ok(());
    }

    Err(BackupCommandError::RestoreNotReady {
        backup_id: report.backup_id.clone(),
        reasons: report.restore_readiness_reasons.clone(),
    })
}

impl From<BackupPreflightError> for BackupCommandError {
    // Preserve the public CLI error variants while delegating preflight work to canic-backup.
    fn from(error: BackupPreflightError) -> Self {
        match error {
            BackupPreflightError::IncompleteJournal {
                backup_id,
                total_artifacts,
                pending_artifacts,
            } => Self::IncompleteJournal {
                backup_id,
                total_artifacts,
                pending_artifacts,
            },
            BackupPreflightError::Io(error) => Self::Io(error),
            BackupPreflightError::Json(error) => Self::Json(error),
            BackupPreflightError::Persistence(error) => Self::Persistence(error),
            BackupPreflightError::RestorePlan(error) => Self::RestorePlan(error),
        }
    }
}
