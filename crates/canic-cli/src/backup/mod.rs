use crate::{output, version_text};
use canic_backup::{
    journal::JournalResumeReport,
    persistence::{BackupIntegrityReport, BackupLayout, PersistenceError},
};
use std::ffi::OsString;
use thiserror::Error as ThisError;

mod options;

pub use options::{BackupStatusOptions, BackupVerifyOptions};

///
/// BackupCommandError
///

#[derive(Debug, ThisError)]
pub enum BackupCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error(
        "backup journal {backup_id} is incomplete: {pending_artifacts}/{total_artifacts} artifacts still require resume work"
    )]
    IncompleteJournal {
        backup_id: String,
        total_artifacts: usize,
        pending_artifacts: usize,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

/// Run a backup subcommand.
pub fn run<I>(args: I) -> Result<(), BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(BackupCommandError::Usage(usage()));
    };

    match command.as_str() {
        "status" => {
            let options = BackupStatusOptions::parse(args)?;
            let report = backup_status(&options)?;
            write_status_report(&options, &report)?;
            enforce_status_requirements(&options, &report)?;
            Ok(())
        }
        "verify" => {
            let options = BackupVerifyOptions::parse(args)?;
            let report = verify_backup(&options)?;
            write_report(&options, &report)?;
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            Ok(())
        }
        _ => Err(BackupCommandError::UnknownOption(command)),
    }
}

/// Summarize a backup journal's resumable state.
pub fn backup_status(
    options: &BackupStatusOptions,
) -> Result<JournalResumeReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    let journal = layout.read_journal()?;
    Ok(journal.resume_report())
}

/// Verify a backup directory's manifest, journal, and durable artifacts.
pub fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.verify_integrity().map_err(BackupCommandError::from)
}

// Ensure a journal status report has no remaining resume work.
fn ensure_complete_status(report: &JournalResumeReport) -> Result<(), BackupCommandError> {
    if report.is_complete {
        return Ok(());
    }

    Err(BackupCommandError::IncompleteJournal {
        backup_id: report.backup_id.clone(),
        total_artifacts: report.total_artifacts,
        pending_artifacts: report.pending_artifacts,
    })
}

// Enforce caller-requested status requirements after the JSON report is written.
fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete {
        return Ok(());
    }

    ensure_complete_status(report)
}

// Write the journal status report to stdout or a requested output file.
fn write_status_report(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the integrity report to stdout or a requested output file.
fn write_report(
    options: &BackupVerifyOptions,
    report: &BackupIntegrityReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Return backup command usage text.
const fn usage() -> &'static str {
    "usage: canic backup <command> [<args>]\n\ncommands:\n  verify      Verify layout, journal agreement, and durable artifact checksums.\n  status      Summarize resumable download journal state."
}

#[cfg(test)]
mod tests;
